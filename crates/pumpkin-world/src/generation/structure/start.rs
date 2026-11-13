//! The `structures` tag of a chunk: the structure starts the chunk owns and the references to
//! the starts of nearby chunks, in the layout of vanilla `SerializableChunkData`.
//!
//! ```text
//! structures: {
//!   starts: { "<structure id>": { id, ChunkX, ChunkZ, references, Children: [piece, ...] } },
//!   References: { "<structure id>": long[] (ChunkPos.pack of each start chunk) }
//! }
//! ```

use std::sync::{Arc, Mutex};

use pumpkin_data::structures::{Structure, StructureKeys, StructureType};
use pumpkin_nbt::{compound::NbtCompound, tag::NbtTag};
use pumpkin_util::{
    BlockDirection,
    math::{block_box::BlockBox, position::BlockPos, vector2::Vector2},
};
use tracing::{error, warn};

use super::{
    piece::StructurePieceType,
    structures::{
        StructurePiece, StructurePieceBase, StructurePiecesCollector, StructurePosition,
        buried_treasure::BuriedTreasurePiece, desert_pyramid::DesertPyramidPiece,
        jungle_temple::JungleTemplePiece, ocean_monument, swamp_hut::SwampHutPiece,
    },
};

/// The chessboard distance in chunks past which vanilla drops a saved reference on load.
const MAX_REFERENCE_DISTANCE: u32 = 8;

/// The id of `structure` in the vanilla `worldgen/structure` registry.
#[must_use]
pub fn structure_id(structure: StructureKeys) -> String {
    format!("minecraft:{}", structure.to_name())
}

/// The path of an identifier in the `minecraft` namespace, as vanilla `Identifier.parse` reads
/// it: an id without a namespace is in `minecraft`. `None` for another namespace.
#[must_use]
pub fn minecraft_path(id: &str) -> Option<&str> {
    match id.split_once(':') {
        Some(("minecraft", path)) => Some(path),
        Some(_) => None,
        None => Some(id),
    }
}

/// Vanilla `ChunkPos.pack`.
#[must_use]
pub const fn pack_chunk_pos(x: i32, z: i32) -> i64 {
    (x as u32 as i64) | ((z as u32 as i64) << 32)
}

/// Vanilla `ChunkPos.unpack`.
#[must_use]
pub const fn unpack_chunk_pos(packed: i64) -> Vector2<i32> {
    Vector2::new(packed as i32, (packed >> 32) as i32)
}

/// The `structures` tag of a chunk, read once when the chunk loads, as vanilla
/// `SerializableChunkData` reads it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedStructures {
    /// The tag as it was read. A save writes it back.
    pub tag: NbtCompound,
    /// The structures with a valid start in this chunk and the boxes of their pieces, or `None`
    /// when the tag has no `starts`.
    pub starts: Option<Vec<(StructureKeys, Vec<BlockBox>)>>,
    /// The structure and start chunk of every valid reference, or `None` when the tag has no
    /// `References`.
    pub references: Option<Vec<(StructureKeys, Vector2<i32>)>>,
}

impl SavedStructures {
    /// Reads the `structures` tag of the chunk at `chunk_pos` and logs what vanilla logs.
    #[must_use]
    pub fn read(tag: NbtCompound, chunk_pos: Vector2<i32>) -> Self {
        let starts = has_saved_starts(&tag).then(|| {
            saved_starts(&tag)
                .into_iter()
                .map(|(structure, start)| (structure, saved_piece_boxes(start)))
                .collect()
        });
        let references = has_saved_references(&tag).then(|| saved_references(&tag, chunk_pos));
        Self {
            tag,
            starts,
            references,
        }
    }

    /// The saved start of `structure`, when `starts` holds a valid one.
    #[must_use]
    pub fn start_tag(&self, structure: StructureKeys) -> Option<&NbtCompound> {
        if !self
            .starts
            .as_ref()?
            .iter()
            .any(|(saved, _)| *saved == structure)
        {
            return None;
        }
        let (_, tag) = self
            .tag
            .get_compound("starts")?
            .child_tags
            .iter()
            .find(|(key, _)| StructureKeys::from_name(key) == Some(structure))?;
        tag.extract_compound()
    }
}

/// The saved starts of the `structures` tag, as vanilla `unpackStructureStart` reads them.
///
/// An unknown structure id is logged and dropped, and so is a start that vanilla
/// `StructureStart.loadStaticStart` rejects.
fn saved_starts(structures: &NbtCompound) -> Vec<(StructureKeys, &NbtCompound)> {
    let Some(starts) = structures.get_compound("starts") else {
        return Vec::new();
    };
    starts
        .child_tags
        .iter()
        .filter_map(|(key, tag)| {
            let Some(structure) = StructureKeys::from_name(key) else {
                error!("Unknown structure start: {key}");
                return None;
            };
            let start = tag.extract_compound()?;
            let id = start.get_string("id").unwrap_or_default();
            if id == "INVALID" {
                return None;
            }
            if StructureKeys::from_name(id).is_none() {
                error!("Unknown structure id: {id}");
                return None;
            }
            Some((structure, start))
        })
        .collect()
}

/// The boxes of the pieces of a saved start, from the `BB` field that every vanilla
/// `StructurePiece` writes. A piece without a valid box is skipped.
fn saved_piece_boxes(start: &NbtCompound) -> Vec<BlockBox> {
    start
        .get_list("Children")
        .unwrap_or_default()
        .iter()
        .filter_map(|child| {
            let &[min_x, min_y, min_z, max_x, max_y, max_z] =
                child.extract_compound()?.get_int_array("BB")?
            else {
                return None;
            };
            Some(BlockBox::new(min_x, min_y, min_z, max_x, max_y, max_z))
        })
        .collect()
}

/// The saved references of the chunk at `chunk_pos`, as vanilla `unpackStructureReferences`
/// reads them.
///
/// A reference to an unknown structure or to a start more than 8 chunks away is logged and
/// dropped.
fn saved_references(
    structures: &NbtCompound,
    chunk_pos: Vector2<i32>,
) -> Vec<(StructureKeys, Vector2<i32>)> {
    let Some(references) = structures.get_compound("References") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, tag) in &references.child_tags {
        let Some(structure) = StructureKeys::from_name(key) else {
            warn!(
                "Found reference to unknown structure '{key}' in chunk [{}, {}], discarding",
                chunk_pos.x, chunk_pos.y
            );
            continue;
        };
        let Some(packed) = tag.extract_long_array() else {
            continue;
        };
        for &packed in packed {
            let start = unpack_chunk_pos(packed);
            if start
                .x
                .abs_diff(chunk_pos.x)
                .max(start.y.abs_diff(chunk_pos.y))
                > MAX_REFERENCE_DISTANCE
            {
                warn!(
                    "Found invalid structure reference [ {key} @ [{}, {}] ] for chunk [{}, {}].",
                    start.x, start.y, chunk_pos.x, chunk_pos.y
                );
                continue;
            }
            out.push((structure, start));
        }
    }
    out
}

/// Vanilla `StructureStart.createTag` for a start of `structure` that Pumpkin generated.
///
/// Returns `None` when a piece type cannot be saved yet. Pumpkin does not count the explorer
/// maps that point to a start, so `references` is 0 unless the caller keeps a saved count.
#[must_use]
pub fn create_tag(
    structure: StructureKeys,
    chunk_pos: Vector2<i32>,
    collector: &StructurePiecesCollector,
) -> Option<NbtCompound> {
    let children = collector
        .pieces
        .iter()
        .map(|piece| create_piece_tag(piece.as_ref()).map(NbtTag::Compound))
        .collect::<Option<Vec<_>>>()?;
    let mut tag = NbtCompound::new();
    tag.put_string("id", structure_id(structure));
    tag.put_int("ChunkX", chunk_pos.x);
    tag.put_int("ChunkZ", chunk_pos.y);
    tag.put_int("references", 0);
    tag.put_list("Children", children);
    Some(tag)
}

/// Vanilla `StructurePiece.createTag`.
fn create_piece_tag(piece: &dyn StructurePieceBase) -> Option<NbtCompound> {
    let base = piece.get_structure_piece();
    let bounding_box = piece.bounding_box();
    let mut tag = NbtCompound::new();
    tag.put_string("id", base.r#type.id().to_string());
    tag.put(
        "BB",
        NbtTag::IntArray(vec![
            bounding_box.min.x,
            bounding_box.min.y,
            bounding_box.min.z,
            bounding_box.max.x,
            bounding_box.max.y,
            bounding_box.max.z,
        ]),
    );
    tag.put_int("O", data_2d_value(base.facing));
    tag.put_int("GD", base.chain_length as i32);
    piece.add_additional_save_data(&mut tag).then_some(tag)
}

/// Vanilla `StructureStart.loadStaticStart` for the saved start of `structure`.
///
/// Returns `None` when a piece fails to load, so that the caller computes the start again.
/// Vanilla `PiecesContainer.load` would drop the piece and place the start without it.
#[must_use]
pub fn load_start(
    structure: StructureKeys,
    tag: &NbtCompound,
    seed: i64,
    sea_level: i32,
) -> Option<StructurePosition> {
    let chunk_pos = Vector2::new(
        tag.get_int("ChunkX").unwrap_or(0),
        tag.get_int("ChunkZ").unwrap_or(0),
    );
    let children = tag.get_list("Children").unwrap_or_default();
    let mut collector = if Structure::get(&structure).structure_type == StructureType::OceanMonument
    {
        let top_piece = children.first()?.extract_compound()?;
        let top_piece = StructurePiece::from_tag(StructurePieceType::OceanMonumentBase, top_piece)?;
        ocean_monument::regenerate_pieces_after_load(chunk_pos, seed, sea_level, &top_piece)
    } else {
        let mut collector = StructurePiecesCollector::default();
        for child in children {
            collector.add_piece(load_piece(child.extract_compound()?)?);
        }
        collector
    };
    if collector.is_empty() {
        return None;
    }
    // Vanilla does not save the position the generator picked; only the generator reads it.
    let bounding_box = collector.get_bounding_box();
    Some(StructurePosition {
        start_pos: BlockPos::new(bounding_box.min.x, bounding_box.min.y, bounding_box.min.z),
        collector: Arc::new(Mutex::new(collector)),
    })
}

/// Vanilla `StructurePieceType.load` for the piece types that Pumpkin can load.
fn load_piece(tag: &NbtCompound) -> Option<Box<dyn StructurePieceBase>> {
    let id = tag.get_string("id").unwrap_or_default();
    let Some(piece_type) = StructurePieceType::from_id(id) else {
        error!("Unknown structure piece id: {id}");
        return None;
    };
    let piece = StructurePiece::from_tag(piece_type, tag)?;
    Some(match piece_type {
        StructurePieceType::BuriedTreasure => Box::new(BuriedTreasurePiece::from_tag(piece)),
        StructurePieceType::DesertTemple => Box::new(DesertPyramidPiece::from_tag(piece, tag)),
        StructurePieceType::JungleTemple => Box::new(JungleTemplePiece::from_tag(piece, tag)),
        StructurePieceType::SwampHut => Box::new(SwampHutPiece::from_tag(piece, tag)),
        _ => return None,
    })
}

/// Vanilla `Direction.get2DDataValue`, with -1 for no orientation as `StructurePiece` writes it.
#[must_use]
pub const fn data_2d_value(facing: Option<BlockDirection>) -> i32 {
    match facing {
        Some(BlockDirection::South) => 0,
        Some(BlockDirection::West) => 1,
        Some(BlockDirection::North) => 2,
        Some(BlockDirection::East) => 3,
        _ => -1,
    }
}

/// Vanilla `Direction.from2DDataValue`, with -1 for no orientation as `StructurePiece` reads it.
#[must_use]
pub const fn direction_from_2d_data_value(value: i32) -> Option<BlockDirection> {
    match value {
        -1 => None,
        value => Some(match (value % 4).abs() {
            0 => BlockDirection::South,
            1 => BlockDirection::West,
            2 => BlockDirection::North,
            _ => BlockDirection::East,
        }),
    }
}

/// Whether the `structures` tag has the `starts` compound, which vanilla always writes.
fn has_saved_starts(structures: &NbtCompound) -> bool {
    matches!(structures.get("starts"), Some(NbtTag::Compound(_)))
}

/// Whether the `structures` tag has the `References` compound, which vanilla always writes.
fn has_saved_references(structures: &NbtCompound) -> bool {
    matches!(structures.get("References"), Some(NbtTag::Compound(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(starts: NbtCompound, references: NbtCompound) -> NbtCompound {
        let mut structures = NbtCompound::new();
        structures.put_compound("starts", starts);
        structures.put_compound("References", references);
        structures
    }

    fn start(id: &str, boxes: &[[i32; 6]]) -> NbtCompound {
        let mut start = NbtCompound::new();
        start.put_string("id", id.to_string());
        start.put_int("ChunkX", 2);
        start.put_int("ChunkZ", -3);
        start.put_int("references", 0);
        start.put_list(
            "Children",
            boxes
                .iter()
                .map(|bb| {
                    let mut piece = NbtCompound::new();
                    piece.put_string("id", "minecraft:jigsaw".to_string());
                    piece.put("BB", NbtTag::IntArray(bb.to_vec()));
                    NbtTag::Compound(piece)
                })
                .collect(),
        );
        start
    }

    #[test]
    fn chunk_pos_packs_like_vanilla() {
        // ChunkPos.pack(-1, 2) == 0x0000_0002_FFFF_FFFF.
        assert_eq!(pack_chunk_pos(-1, 2), 0x0000_0002_FFFF_FFFF);
        for (x, z) in [
            (0, 0),
            (-1, 2),
            (i32::MIN, i32::MAX),
            (1_875_000, -1_875_000),
        ] {
            assert_eq!(unpack_chunk_pos(pack_chunk_pos(x, z)), Vector2::new(x, z));
        }
    }

    #[test]
    fn references_drop_unknown_structures_and_far_starts() {
        let mut references = NbtCompound::new();
        references.put(
            "minecraft:village_plains",
            NbtTag::LongArray(vec![pack_chunk_pos(2, -3), pack_chunk_pos(10, 0)]),
        );
        references.put(
            "minecraft:mineshaft",
            NbtTag::LongArray(vec![pack_chunk_pos(11, 0)]),
        );
        references.put(
            "othermod:tower",
            NbtTag::LongArray(vec![pack_chunk_pos(2, -3)]),
        );
        let structures = tag(NbtCompound::new(), references);

        let mut found = SavedStructures::read(structures, Vector2::new(2, 0))
            .references
            .expect("the tag has references");
        found.sort_by_key(|(structure, start)| (structure.to_name(), start.x));
        assert_eq!(
            found,
            [
                (StructureKeys::VillagePlains, Vector2::new(2, -3)),
                (StructureKeys::VillagePlains, Vector2::new(10, 0)),
            ]
        );
    }

    #[test]
    fn starts_drop_unknown_and_invalid_ids() {
        let mut starts = NbtCompound::new();
        starts.put_compound(
            "minecraft:village_plains",
            start("minecraft:village_plains", &[[0, 60, 0, 9, 70, 9]]),
        );
        starts.put_compound("minecraft:igloo", start("INVALID", &[]));
        starts.put_compound("othermod:tower", start("othermod:tower", &[]));
        let structures = tag(starts, NbtCompound::new());

        let saved = SavedStructures::read(structures, Vector2::new(2, -3));
        assert_eq!(
            saved.starts,
            Some(vec![(
                StructureKeys::VillagePlains,
                vec![BlockBox::new(0, 60, 0, 9, 70, 9)]
            )])
        );
        assert_eq!(saved.references, Some(Vec::new()));
        let empty = SavedStructures::read(NbtCompound::new(), Vector2::new(2, -3));
        assert_eq!((empty.starts, empty.references), (None, None));
    }
}
