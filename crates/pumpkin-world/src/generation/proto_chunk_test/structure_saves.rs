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

/// The vanilla dump starts of the structures whose pieces write no fields of their own.
fn base_only_starts() -> Vec<StartDump> {
    let dump: StructureStartsDump = serde_json::from_str(include_str!(
        "../../../../../assets/tests/structure_starts.json"
    ))
    .expect("dump parses");
    dump.overworld
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
