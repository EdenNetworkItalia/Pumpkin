use std::io::Cursor;

use crate::chunk::ChunkData;
use crate::chunk_system::{chunk_state::Chunk, chunk_state::StagedChunkEnum};
use crate::generation::{
    generator::WorldGenerator,
    get_world_gen,
    proto_chunk::ProtoChunk,
    structure::start::{self, SavedStructures},
    structure::structures::StructureInstance,
};
use pumpkin_config::lighting::LightingEngineConfig;
use pumpkin_data::{chunk::ChunkStatus, dimension::Dimension, structures::StructureKeys};
use pumpkin_nbt::{compound::NbtCompound, deserializer::NbtReadHelperJava, tag::NbtTag};
use pumpkin_util::world_seed::Seed;

/// The vanilla dump of `structure_starts.rs`, generated with seed 0.
#[derive(serde::Deserialize)]
struct StructureStartsDump {
    overworld: DimensionDump,
    the_nether: DimensionDump,
}

#[derive(serde::Deserialize)]
struct DimensionDump {
    chunks: Vec<ChunkDump>,
}

#[derive(serde::Deserialize)]
struct ChunkDump {
    starts: Vec<StartDump>,
}

#[derive(serde::Deserialize)]
struct StartDump {
    structure: String,
    chunk_x: i32,
    chunk_z: i32,
    references: i32,
    pieces: Vec<PieceDump>,
}

#[derive(serde::Deserialize)]
struct PieceDump {
    #[serde(rename = "type")]
    piece_type: String,
    bounding_box: BoxDump,
    orientation: i32,
    gen_depth: i32,
    /// The fields of a jigsaw piece, as the vanilla codecs write them in JSON.
    piece_data: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct BoxDump {
    min_x: i32,
    min_y: i32,
    min_z: i32,
    max_x: i32,
    max_y: i32,
    max_z: i32,
}

/// The start tag that vanilla `StructureStart.createTag` writes for `start`, for piece types
/// whose `addAdditionalSaveData` writes nothing.
fn vanilla_start_tag(start: &StartDump) -> NbtCompound {
    let children = start
        .pieces
        .iter()
        .map(|piece| {
            let bb = &piece.bounding_box;
            let mut tag = NbtCompound::new();
            tag.put_string("id", piece.piece_type.clone());
            tag.put(
                "BB",
                NbtTag::IntArray(vec![
                    bb.min_x, bb.min_y, bb.min_z, bb.max_x, bb.max_y, bb.max_z,
                ]),
            );
            tag.put_int("O", piece.orientation);
            tag.put_int("GD", piece.gen_depth);
            if let Some(data) = &piece.piece_data {
                add_jigsaw_fields(&mut tag, data);
            }
            NbtTag::Compound(tag)
        })
        .collect();
    let mut tag = NbtCompound::new();
    tag.put_string("id", start.structure.clone());
    tag.put_int("ChunkX", start.chunk_x);
    tag.put_int("ChunkZ", start.chunk_z);
    tag.put_int("references", start.references);
    tag.put_list("Children", children);
    tag
}

/// The NBT that vanilla `NbtOps` gives for a value that the vanilla codecs write as `json`.
fn json_to_nbt(json: &serde_json::Value) -> NbtTag {
    match json {
        serde_json::Value::String(string) => NbtTag::String(string.as_str().into()),
        serde_json::Value::Number(number) => NbtTag::Int(number.as_i64().expect("integer") as i32),
        serde_json::Value::Array(values) => NbtTag::List(values.iter().map(json_to_nbt).collect()),
        serde_json::Value::Object(fields) => {
            let mut compound = NbtCompound::new();
            for (name, value) in fields {
                compound.put(name, json_to_nbt(value));
            }
            NbtTag::Compound(compound)
        }
        other => panic!("no NBT for {other}"),
    }
}

/// The fields of vanilla `PoolElementStructurePiece.addAdditionalSaveData` from the dump.
fn add_jigsaw_fields(tag: &mut NbtCompound, data: &serde_json::Value) {
    let position = data["position"].as_array().expect("position");
    for (name, value) in ["PosX", "PosY", "PosZ"].into_iter().zip(position) {
        tag.put(name, json_to_nbt(value));
    }
    for name in [
        "ground_level_delta",
        "pool_element",
        "rotation",
        "junctions",
    ] {
        tag.put(name, json_to_nbt(&data[name]));
    }
    if data["liquid_settings"] != "apply_waterlogging" {
        tag.put("liquid_settings", json_to_nbt(&data["liquid_settings"]));
    }
}

/// Asserts that two start tags are equal, naming the first piece and fields that differ.
fn assert_same_start(written: &NbtCompound, expected: &NbtCompound, label: &str) {
    let pieces = |start: &NbtCompound| {
        start
            .get_list("Children")
            .unwrap_or_default()
            .iter()
            .filter_map(NbtTag::extract_compound)
            .cloned()
            .collect::<Vec<_>>()
    };
    let (written_pieces, expected_pieces) = (pieces(written), pieces(expected));
    for (index, (written, expected)) in written_pieces.iter().zip(&expected_pieces).enumerate() {
        let differing: Vec<_> = written
            .child_tags
            .keys()
            .chain(expected.child_tags.keys())
            .filter(|name| written.get(name) != expected.get(name))
            .collect();
        assert!(
            differing.is_empty(),
            "{label}: piece {index} differs in {differing:?}: {written:?} != {expected:?}"
        );
    }
    assert_eq!(
        written_pieces.len(),
        expected_pieces.len(),
        "{label}: piece count"
    );
    assert_eq!(written, expected, "{label}");
}

/// The `structures` tag of the Anvil save of the chunk at `chunk_x`, `chunk_z` after the
/// structure stages.
fn saved_structures(world_gen: &WorldGenerator, chunk_x: i32, chunk_z: i32) -> NbtCompound {
    let WorldGenerator::Noise(generator) = world_gen else {
        unreachable!()
    };
    let mut proto = ProtoChunk::new(chunk_x, chunk_z, world_gen);
    proto.step_to_biomes(generator);
    proto.set_structure_starts(generator);
    proto.set_structure_references(generator);
    let mut staged = Chunk::Proto(Box::new(proto));
    staged.upgrade_to_level_chunk(&Dimension::OVERWORLD, &LightingEngineConfig::Default);
    let Chunk::Level(chunk) = staged else {
        unreachable!()
    };
    let bytes = chunk.internal_to_bytes();
    let nbt = pumpkin_nbt::Nbt::read(&mut NbtReadHelperJava::new(&mut Cursor::new(&bytes[..])))
        .expect("saved chunk parses");
    nbt.root_tag
        .get_compound("structures")
        .cloned()
        .unwrap_or_default()
}

fn dump() -> StructureStartsDump {
    serde_json::from_str(include_str!(
        "../../../../../assets/tests/structure_starts.json"
    ))
    .expect("dump parses")
}

/// The vanilla dump starts of the structures whose pieces write no fields of their own.
fn base_only_starts() -> Vec<StartDump> {
    dump()
        .overworld
        .chunks
        .into_iter()
        .flat_map(|chunk| chunk.starts)
        .filter(|start| {
            matches!(
                start.structure.as_str(),
                "minecraft:buried_treasure" | "minecraft:monument"
            )
        })
        .collect()
}

/// A chunk that Pumpkin generates saves its starts as vanilla does, and a reference to each.
#[test]
fn base_only_starts_are_saved_like_vanilla() {
    let world_gen = get_world_gen(
        Seed(0),
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    );
    let starts = base_only_starts();
    assert!(starts.len() >= 2);
    for start in &starts {
        let structures = saved_structures(&world_gen, start.chunk_x, start.chunk_z);
        let saved = structures
            .get_compound("starts")
            .and_then(|starts| starts.get_compound(&start.structure));
        assert_eq!(
            saved,
            Some(&vanilla_start_tag(start)),
            "{}",
            start.structure
        );
        let references = structures
            .get_compound("References")
            .and_then(|references| references.get_long_array(&start.structure));
        assert!(
            references.is_some_and(|references| references
                .contains(&start::pack_chunk_pos(start.chunk_x, start.chunk_z))),
            "{} references its own start",
            start.structure
        );
    }
}

/// A saved start loads back into the same start, and an ocean monument builds the same rooms
/// again from its first piece, as vanilla `regeneratePiecesAfterLoad` does.
#[test]
fn base_only_starts_load_back() {
    let world_gen = get_world_gen(
        Seed(0),
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    );
    let WorldGenerator::Noise(generator) = &*world_gen else {
        unreachable!()
    };
    for start in base_only_starts() {
        let structure = StructureKeys::from_name(&start.structure).expect("known structure");
        let tag = vanilla_start_tag(&start);
        let loaded = start::load_start(structure, &tag, 0, generator.settings.sea_level)
            .expect("the start loads");
        let collector = loaded
            .collector
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let saved_again = start::create_tag(
            structure,
            pumpkin_util::math::vector2::Vector2::new(start.chunk_x, start.chunk_z),
            &collector,
        );
        assert_eq!(saved_again, Some(tag), "{}", start.structure);
    }
}

/// A chunk that resumes generation places the saved start, not the one that the generator
/// computes, and the chunks around it reference the saved start too.
#[test]
fn resumed_chunk_reads_saved_start_and_references() {
    let world_gen = get_world_gen(
        Seed(0),
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    );
    let start = base_only_starts()
        .into_iter()
        .find(|start| start.structure == "minecraft:buried_treasure")
        .expect("the dump has a buried treasure");
    let start_chunk = pumpkin_util::math::vector2::Vector2::new(start.chunk_x, start.chunk_z);

    // The saved piece is 50 blocks lower than the one the generator computes.
    let mut saved_start = vanilla_start_tag(&start);
    if let Some(NbtTag::List(children)) = saved_start.child_tags.get_mut("Children")
        && let Some(NbtTag::Compound(piece)) = children.first_mut()
        && let Some(NbtTag::IntArray(bb)) = piece.child_tags.get_mut("BB")
    {
        bb[1] -= 50;
        bb[4] -= 50;
    }
    saved_start.put_int("references", 2);
    let mut starts = NbtCompound::new();
    starts.put_compound(&start.structure, saved_start);
    let mut references = NbtCompound::new();
    references.put(
        &start.structure,
        NbtTag::LongArray(vec![start::pack_chunk_pos(start_chunk.x, start_chunk.y)]),
    );
    let mut structures = NbtCompound::new();
    structures.put_compound("starts", starts);
    structures.put_compound("References", references);
    let structures = SavedStructures::read(structures, start_chunk);
    let saved_box = structures.starts.as_ref().expect("the tag has starts")[0].1[0];

    let start_chunk_data = ChunkData {
        status: ChunkStatus::StructureStarts,
        structures: structures.clone(),
        ..ChunkData::empty(start_chunk.x, start_chunk.y)
    };
    let resumed = ProtoChunk::from_chunk_data(&start_chunk_data, &world_gen);
    assert_eq!(resumed.stage, StagedChunkEnum::StructureStart);
    let Some(StructureInstance::Start(position)) =
        resumed.structure_starts.get(&StructureKeys::BuriedTreasure)
    else {
        panic!("the saved start is placed");
    };
    assert_eq!(position.get_bounding_box(), saved_box);
    let saved_again = resumed.structure_data();
    let saved_again = saved_again
        .get_compound("starts")
        .and_then(|starts| starts.get_compound(&start.structure))
        .expect("the start is saved again");
    assert_eq!(
        saved_again.get_int("references"),
        Some(2),
        "the save keeps the explorer map count"
    );

    // The start chunk itself, resumed one stage later, references the saved start.
    let referencing = ChunkData {
        status: ChunkStatus::StructureReferences,
        structures,
        ..ChunkData::empty(start_chunk.x, start_chunk.y)
    };
    let resumed = ProtoChunk::from_chunk_data(&referencing, &world_gen);
    let references: Vec<_> = resumed
        .structure_references
        .iter()
        .filter(|(structure, _, _)| *structure == StructureKeys::BuriedTreasure)
        .collect();
    assert_eq!(references.len(), 1);
    let (_, reference_start, collector) = references[0];
    assert_eq!(*reference_start, start_chunk);
    assert_eq!(
        collector
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_bounding_box(),
        saved_box
    );
}

/// A start chunk that loads again keeps the cached start, which its neighbours may have moved
/// since the chunk was saved: the stale saved copy would put the structure at two heights.
#[test]
fn reloaded_start_chunk_keeps_the_cached_start() {
    let world_gen = get_world_gen(
        Seed(0),
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    );
    let WorldGenerator::Noise(generator) = &*world_gen else {
        unreachable!()
    };
    let start = base_only_starts()
        .into_iter()
        .find(|start| start.structure == "minecraft:buried_treasure")
        .expect("the dump has a buried treasure");
    let start_chunk = pumpkin_util::math::vector2::Vector2::new(start.chunk_x, start.chunk_z);

    // The start chunk computes its start and is saved before its neighbours place the pieces.
    let mut proto = ProtoChunk::new(start_chunk.x, start_chunk.y, &world_gen);
    proto.step_to_biomes(generator);
    proto.set_structure_starts(generator);
    let Some(StructureInstance::Start(cached)) =
        proto.structure_starts.get(&StructureKeys::BuriedTreasure)
    else {
        panic!("the start chunk has the start");
    };
    let cached = cached.collector.clone();
    let stale = SavedStructures::read(proto.structure_data(), start_chunk);
    let stale_box = stale.starts.as_ref().expect("the start is saved")[0].1[0];

    // A neighbour moves the pieces, as a height adjustment does.
    let adjusted_box = {
        let mut collector = cached
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        collector.shift(7);
        collector.get_bounding_box()
    };
    assert_ne!(adjusted_box, stale_box);

    // The start chunk loads again, as a proto chunk that resumes and as a full chunk.
    for status in [ChunkStatus::StructureStarts, ChunkStatus::Full] {
        let chunk_data = ChunkData {
            status,
            structures: stale.clone(),
            ..ChunkData::empty(start_chunk.x, start_chunk.y)
        };
        if status == ChunkStatus::Full {
            ProtoChunk::register_saved_starts(&chunk_data, &world_gen);
        } else {
            let resumed = ProtoChunk::from_chunk_data(&chunk_data, &world_gen);
            let Some(StructureInstance::Start(position)) =
                resumed.structure_starts.get(&StructureKeys::BuriedTreasure)
            else {
                panic!("the resumed chunk has the start");
            };
            assert!(
                std::sync::Arc::ptr_eq(&position.collector, &cached),
                "{status:?}"
            );
        }
        let Some(Some(position)) = generator.global_structure_cache.cached_structure_start(
            StructureKeys::BuriedTreasure,
            start_chunk.x,
            start_chunk.y,
        ) else {
            panic!("the start stays cached");
        };
        assert!(
            std::sync::Arc::ptr_eq(&position.collector, &cached),
            "{status:?}"
        );
        assert_eq!(position.get_bounding_box(), adjusted_box, "{status:?}");
    }
}

/// A start tag with one piece of `piece_id` and the given fields of its own.
fn one_piece_start(
    structure: &str,
    piece_id: &str,
    ints: &[(&str, i32)],
    bools: &[(&str, bool)],
) -> NbtCompound {
    let mut piece = NbtCompound::new();
    piece.put_string("id", piece_id.to_string());
    piece.put("BB", NbtTag::IntArray(vec![0, 70, 0, 20, 84, 20]));
    piece.put_int("O", 3);
    piece.put_int("GD", 0);
    for &(name, value) in ints {
        piece.put_int(name, value);
    }
    for &(name, value) in bools {
        piece.put_bool(name, value);
    }
    let mut start = NbtCompound::new();
    start.put_string("id", structure.to_string());
    start.put_int("ChunkX", 0);
    start.put_int("ChunkZ", 0);
    start.put_int("references", 0);
    start.put_list("Children", vec![NbtTag::Compound(piece)]);
    start
}

/// Scattered feature pieces load back the fields that vanilla `ScatteredFeaturePiece` and its
/// subclasses write, and save them again unchanged.
#[test]
fn scattered_feature_starts_load_back() {
    let scattered = |width, height, depth| {
        [
            ("Width", width),
            ("Height", height),
            ("Depth", depth),
            ("HPos", 68),
        ]
    };
    for (structure, tag) in [
        (
            StructureKeys::DesertPyramid,
            one_piece_start(
                "minecraft:desert_pyramid",
                "minecraft:tedp",
                &scattered(21, 15, 21),
                &[
                    ("hasPlacedChest0", true),
                    ("hasPlacedChest1", false),
                    ("hasPlacedChest2", true),
                    ("hasPlacedChest3", false),
                ],
            ),
        ),
        (
            StructureKeys::JunglePyramid,
            one_piece_start(
                "minecraft:jungle_pyramid",
                "minecraft:tejp",
                &scattered(12, 10, 15),
                &[
                    ("placedMainChest", true),
                    ("placedHiddenChest", false),
                    ("placedTrap1", true),
                    ("placedTrap2", false),
                ],
            ),
        ),
        (
            StructureKeys::SwampHut,
            one_piece_start(
                "minecraft:swamp_hut",
                "minecraft:tesh",
                &scattered(7, 7, 9),
                &[("Witch", true), ("Cat", false)],
            ),
        ),
    ] {
        let loaded = start::load_start(structure, &tag, 0, 63).expect("the start loads");
        let collector = loaded
            .collector
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let saved_again = start::create_tag(
            structure,
            pumpkin_util::math::vector2::Vector2::new(0, 0),
            &collector,
        );
        assert_eq!(saved_again, Some(tag), "{structure:?}");
    }
}

/// `start` without the piece fields in `fields`.
fn without_piece_fields(start: &NbtCompound, fields: &[&str]) -> NbtCompound {
    let mut start = start.clone();
    if let Some(NbtTag::List(children)) = start.child_tags.get_mut("Children") {
        for child in children {
            if let NbtTag::Compound(piece) = child {
                for field in fields {
                    piece.child_tags.remove(*field);
                }
            }
        }
    }
    start
}

/// Template structure starts are written with the piece fields that vanilla 26.3 writes. The
/// fixture holds starts that the vanilla server saved for this seed, keyed by structure.
/// Vanilla saved them after placement, which moves some pieces to the terrain height, and the
/// generated starts stop before placement, so the fields that placement changes are left out.
/// The vanilla starts, placement fields included, load back and save unchanged.
#[test]
fn template_starts_match_vanilla() {
    let expected = pumpkin_nbt::nbt_compress::read_gzip_compound_tag(Cursor::new(include_bytes!(
        "../../../../../assets/tests/template_structure_starts.nbt"
    )))
    .expect("fixture parses");
    let seed = Seed(1_782_124_772_053_846_960);
    // Vanilla takes a loaded template piece's box from the template, and an igloo restores its
    // template position after placement but keeps the moved box.
    for (structure, dimension, chunk_x, chunk_z, placement_fields, reloaded_fields) in [
        (
            "minecraft:igloo",
            Dimension::OVERWORLD,
            134,
            -224,
            &["BB"][..],
            &["BB"][..],
        ),
        (
            "minecraft:shipwreck",
            Dimension::OVERWORLD,
            0,
            -164,
            &["BB", "TPY", "height_adjusted"][..],
            &[][..],
        ),
        (
            "minecraft:shipwreck_beached",
            Dimension::OVERWORLD,
            24,
            -142,
            &["BB", "TPY", "height_adjusted"][..],
            &[][..],
        ),
        (
            "minecraft:ocean_ruin_warm",
            Dimension::OVERWORLD,
            29,
            -135,
            &["BB", "TPY"][..],
            &[][..],
        ),
        (
            "minecraft:ocean_ruin_cold",
            Dimension::OVERWORLD,
            40,
            -176,
            &["BB", "TPY"][..],
            &[][..],
        ),
        (
            "minecraft:end_city",
            Dimension::THE_END,
            88,
            24,
            &[][..],
            &[][..],
        ),
    ] {
        let world_gen = get_world_gen(seed, dimension, false, Vec::new(), String::new());
        let WorldGenerator::Noise(generator) = &*world_gen else {
            unreachable!()
        };
        let mut proto = ProtoChunk::new(chunk_x, chunk_z, &world_gen);
        proto.step_to_biomes(generator);
        proto.set_structure_starts(generator);
        let structures = proto.structure_data();
        let written = structures
            .get_compound("starts")
            .and_then(|starts| starts.get_compound(structure))
            .map(|start| without_piece_fields(start, placement_fields));
        let vanilla = expected
            .get_compound(structure)
            .expect("fixture has the start");
        assert_eq!(
            written,
            Some(without_piece_fields(vanilla, placement_fields)),
            "{structure}"
        );

        let key = StructureKeys::from_name(structure).expect("known structure");
        let loaded = start::load_start(key, vanilla, seed.0 as i64, generator.settings.sea_level)
            .expect("the vanilla start loads");
        let collector = loaded
            .collector
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let saved_again = start::create_tag(
            key,
            pumpkin_util::math::vector2::Vector2::new(chunk_x, chunk_z),
            &collector,
        );
        assert_eq!(
            saved_again.map(|start| without_piece_fields(&start, reloaded_fields)),
            Some(without_piece_fields(vanilla, reloaded_fields)),
            "{structure} loads back"
        );
    }
}

/// The structure families of the vanilla dump whose own piece fields the dump does not hold.
const DUMPED_FAMILIES: [&str; 2] = ["minecraft:ruined_portal", "minecraft:nether_fossil"];

/// The first starts of each family in `DUMPED_FAMILIES` and each dimension: the written pieces
/// have the common fields of the dump, and every start loads back and saves unchanged.
#[test]
fn dumped_starts_load_back() {
    let dump = dump();
    let mut checked = std::collections::HashMap::<(usize, &str), usize>::new();
    for (dimension_index, (dimension, dimension_dump)) in [
        (Dimension::OVERWORLD, dump.overworld),
        (Dimension::THE_NETHER, dump.the_nether),
    ]
    .into_iter()
    .enumerate()
    {
        let world_gen = get_world_gen(Seed(0), dimension, false, Vec::new(), String::new());
        let WorldGenerator::Noise(generator) = &*world_gen else {
            unreachable!()
        };
        for start in dimension_dump
            .chunks
            .into_iter()
            .flat_map(|chunk| chunk.starts)
        {
            let Some(family) = DUMPED_FAMILIES
                .into_iter()
                .find(|family| start.structure.starts_with(family))
            else {
                continue;
            };
            let count = checked.entry((dimension_index, family)).or_default();
            if *count == 8 {
                continue;
            }
            *count += 1;
            let mut proto = ProtoChunk::new(start.chunk_x, start.chunk_z, &world_gen);
            proto.step_to_biomes(generator);
            proto.set_structure_starts(generator);
            let structures = proto.structure_data();
            let written = structures
                .get_compound("starts")
                .and_then(|starts| starts.get_compound(&start.structure))
                .expect("the start is written");
            let common = ["id", "BB", "O", "GD"];
            let own_fields = |start: &NbtCompound| {
                let mut start = start.clone();
                if let Some(NbtTag::List(children)) = start.child_tags.get_mut("Children") {
                    for child in children {
                        if let NbtTag::Compound(piece) = child {
                            piece.child_tags.retain(|name, _| common.contains(&&**name));
                        }
                    }
                }
                start
            };
            assert_eq!(
                own_fields(written),
                vanilla_start_tag(&start),
                "{}",
                start.structure
            );

            let key = StructureKeys::from_name(&start.structure).expect("known structure");
            let loaded = start::load_start(key, written, 0, generator.settings.sea_level)
                .expect("the start loads");
            let collector = loaded
                .collector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let chunk = pumpkin_util::math::vector2::Vector2::new(start.chunk_x, start.chunk_z);
            assert_eq!(
                start::create_tag(key, chunk, &collector).as_ref(),
                Some(written),
                "{} loads back",
                start.structure
            );
        }
    }
    for family in DUMPED_FAMILIES {
        assert!(
            checked.keys().any(|(_, checked)| *checked == family),
            "the dump has no {family} start"
        );
    }
}

/// The jigsaw starts of the vanilla dump are written with the fields of every piece that the
/// vanilla codecs write, and load back.
#[test]
fn jigsaw_starts_match_vanilla_dump() {
    let dump = dump();
    let mut checked = 0;
    for (dimension, dimension_dump) in [
        (Dimension::OVERWORLD, dump.overworld),
        (Dimension::THE_NETHER, dump.the_nether),
    ] {
        let world_gen = get_world_gen(Seed(0), dimension, false, Vec::new(), String::new());
        let WorldGenerator::Noise(generator) = &*world_gen else {
            unreachable!()
        };
        let starts = dimension_dump
            .chunks
            .into_iter()
            .flat_map(|chunk| chunk.starts)
            .filter(|start| start.pieces[0].piece_type == "minecraft:jigsaw");
        for start in starts {
            let mut proto = ProtoChunk::new(start.chunk_x, start.chunk_z, &world_gen);
            proto.step_to_biomes(generator);
            proto.set_structure_starts(generator);
            let structures = proto.structure_data();
            let written = structures
                .get_compound("starts")
                .and_then(|starts| starts.get_compound(&start.structure))
                .expect("the start is written");
            assert_same_start(written, &vanilla_start_tag(&start), &start.structure);

            let key = StructureKeys::from_name(&start.structure).expect("known structure");
            let loaded = start::load_start(key, written, 0, generator.settings.sea_level)
                .expect("the start loads");
            let collector = loaded
                .collector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let chunk = pumpkin_util::math::vector2::Vector2::new(start.chunk_x, start.chunk_z);
            let saved_again = start::create_tag(key, chunk, &collector).expect("the start saves");
            // Vanilla takes a loaded jigsaw piece's box from its element, without the growth
            // of the village expansion hack.
            assert_same_start(
                &without_piece_fields(&saved_again, &["BB"]),
                &without_piece_fields(written, &["BB"]),
                &format!("{} loads back", start.structure),
            );
            checked += 1;
        }
    }
    assert!(checked >= 10);
}

/// The village start that the vanilla 26.3 server saved for chunk 36 -126 of this seed.
#[test]
fn village_start_matches_vanilla_save() {
    let expected = pumpkin_nbt::nbt_compress::read_gzip_compound_tag(Cursor::new(include_bytes!(
        "../../../../../assets/tests/village_plains_start_36_-126.nbt"
    )))
    .expect("fixture parses");
    let world_gen = get_world_gen(
        Seed(1_782_124_772_053_846_960),
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    );
    let WorldGenerator::Noise(generator) = &*world_gen else {
        unreachable!()
    };
    let mut proto = ProtoChunk::new(36, -126, &world_gen);
    proto.step_to_biomes(generator);
    proto.set_structure_starts(generator);
    let structures = proto.structure_data();
    let written = structures
        .get_compound("starts")
        .and_then(|starts| starts.get_compound("minecraft:village_plains"))
        .expect("the village start is written");
    assert_same_start(written, &expected, "village");
}
