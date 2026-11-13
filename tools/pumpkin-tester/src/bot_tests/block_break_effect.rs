use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use pumpkin_core::{server::Server, world::BlockFlags};
use pumpkin_data::{Block, dimension::Dimension, packet::CURRENT_MC_VERSION, world::WorldEvent};
use pumpkin_protocol::{
    java::client::play::CWorldEvent, packet::MultiVersionJavaPacket, ser::NetworkReadExt,
};
use pumpkin_util::math::position::BlockPos;

use crate::bot::{HeadlessBot, MAX_POLL_TICKS};

/// Vanilla `Block.spawnDestroyParticles` sends level event 2001, the break particles and the
/// break sound. Level event 2014 has the particles only and belongs to `StrawBedBlock`.
pub async fn run_block_break_effect_test(
    server: &Arc<Server>,
    bot1: &mut HeadlessBot,
    bot2: &mut HeadlessBot,
    summary_lines: &mut Vec<String>,
) -> Result<(), String> {
    let start = Instant::now();
    let world = server.get_world_from_dimension(&Dimension::OVERWORLD);
    let pos = BlockPos::new(
        bot1.position.x.floor() as i32 - 1,
        bot1.position.y.floor() as i32 + 1,
        bot1.position.z.floor() as i32 + 1,
    );
    world.set_block_state(
        &pos,
        Block::DIAMOND_BLOCK.default_state.id,
        BlockFlags::NOTIFY_ALL,
    );

    // TestBot1 is in creative, so this breaks the block on the server. TestBot2 only gets the
    // break effect from the server.
    bot1.start_digging(pos, 3).await?;

    let mut block_broken = false;
    for _ in 0..MAX_POLL_TICKS {
        tokio::time::sleep(Duration::from_millis(5)).await;
        server.tick();
        if world.get_block(&pos).id == Block::AIR.id {
            block_broken = true;
            break;
        }
    }
    if !block_broken {
        return Err("TestBot1 did not break the diamond block in creative".to_string());
    }

    let (event, data) = wait_for_world_event_at(bot2, pos).await?;

    let expected_event = WorldEvent::ParticlesAndSoundDestroyBlock as i32;
    if event != expected_event {
        return Err(format!(
            "Breaking a block sent level event {event} to the other player, expected {expected_event}"
        ));
    }
    let expected_data = i32::from(Block::DIAMOND_BLOCK.default_state.id.as_u16());
    if data != expected_data {
        return Err(format!(
            "Break effect carried block state {data}, expected {expected_data}"
        ));
    }

    summary_lines.push(format!(
        "[PASS] bot:block_break_level_event ({}ms)",
        start.elapsed().as_millis()
    ));
    Ok(())
}

/// Reads packets until the level event at `pos` and returns its event id and data.
async fn wait_for_world_event_at(
    bot: &mut HeadlessBot,
    pos: BlockPos,
) -> Result<(i32, i32), String> {
    let level_event_id = CWorldEvent::to_id(CURRENT_MC_VERSION);
    loop {
        let packet = bot.next_packet().await?;
        if packet.id != level_event_id {
            continue;
        }
        let mut read = &packet.payload[..];
        let event = read.get_i32_be().map_err(|err| err.to_string())?;
        let location = read
            .get_block_pos(&CURRENT_MC_VERSION)
            .map_err(|err| err.to_string())?;
        let data = read.get_i32_be().map_err(|err| err.to_string())?;
        if location == pos {
            return Ok((event, data));
        }
    }
}
