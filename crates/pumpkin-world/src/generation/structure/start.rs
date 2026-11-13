//! The `structures` tag of a chunk: the structure starts the chunk owns and the references to
//! the starts of nearby chunks, in the layout of vanilla `SerializableChunkData`.
//!
//! ```text
//! structures: {
//!   starts: { "<structure id>": { id, ChunkX, ChunkZ, references, Children: [piece, ...] } },
//!   References: { "<structure id>": long[] (ChunkPos.pack of each start chunk) }
//! }
//! ```

use pumpkin_data::structures::StructureKeys;
use pumpkin_nbt::{compound::NbtCompound, tag::NbtTag};
use pumpkin_util::math::{block_box::BlockBox, vector2::Vector2};
use tracing::{error, warn};

/// The chessboard distance in chunks past which vanilla drops a saved reference on load.
const MAX_REFERENCE_DISTANCE: u32 = 8;

/// The id of `structure` in the vanilla `worldgen/structure` registry.
#[must_use]
pub fn structure_id(structure: StructureKeys) -> String {
    format!("minecraft:{}", structure.to_name())
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
