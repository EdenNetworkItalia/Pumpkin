use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use pumpkin_core::{server::Server, world::BlockFlags};
use pumpkin_data::{
    Block, dimension::Dimension, item::Item, item_stack::ItemStack, packet::CURRENT_MC_VERSION,
};
use pumpkin_protocol::{
    java::client::play::{CSwingArm, CWorldEvent},
    packet::MultiVersionJavaPacket,
    ser::NetworkReadExt,
};
use pumpkin_util::{Hand, math::position::BlockPos};

use crate::bot::{HeadlessBot, MAX_POLL_TICKS};

/// Vanilla `ServerGamePacketListenerImpl.handleUseItemOn` swings the arm for a `Success`
/// result with `SwingSource.PREDICTED`. The swinging player predicts it, so only the other
/// players receive the swing. Using a trapdoor and placing a block with `BlockItem.useOn`
/// both return `Success`.
pub async fn run_use_block_swing_test(
    server: &Arc<Server>,
    bot1: &mut HeadlessBot,
    bot2: &mut HeadlessBot,
    summary_lines: &mut Vec<String>,
) -> Result<(), String> {
    let start = Instant::now();
    let world = server.get_world_from_dimension(&Dimension::OVERWORLD);
    let Some(player1) = world.get_player_by_name("TestBot1") else {
        return Err("Could not find 'TestBot1' in Overworld".to_string());
    };
    let swinger_id = player1.entity_id();

    let base_x = bot1.position.x.floor() as i32;
    let base_y = bot1.position.y.floor() as i32;
    let base_z = bot1.position.z.floor() as i32;
    let trapdoor_pos = BlockPos::new(base_x + 1, base_y + 1, base_z + 2);
    let marker_pos = |dy: i32| BlockPos::new(base_x - 1, base_y + 1 + dy, base_z + 2);
    world.set_block_state(
        &trapdoor_pos,
        Block::OAK_TRAPDOOR.default_state.id,
        BlockFlags::NOTIFY_ALL,
    );
    let place_target = BlockPos::new(base_x + 1, base_y + 1, base_z + 3);
    world.set_block_state(
        &place_target,
        Block::DIAMOND_BLOCK.default_state.id,
        BlockFlags::NOTIFY_ALL,
    );
    // The terrain around the bot is random: the spot above the target must be free.
    world.set_block_state(
        &place_target.up(),
        Block::AIR.default_state.id,
        BlockFlags::NOTIFY_ALL,
    );
    for dy in 0..3 {
        world.set_block_state(
            &marker_pos(dy),
            Block::DIAMOND_BLOCK.default_state.id,
            BlockFlags::NOTIFY_ALL,
        );
    }

    // TestBot1 is in creative: breaking a marker block shows how far the server has got. The
    // first one drains what TestBot2 got from the earlier tests.
    bot1.start_digging(marker_pos(0), 4).await?;
    tick_until_broken(server, marker_pos(0)).await?;
    count_swings_until_break_effect(bot2, swinger_id, marker_pos(0)).await?;

    bot1.use_block(trapdoor_pos, 5).await?;
    bot1.start_digging(marker_pos(1), 6).await?;
    tick_until_broken(server, marker_pos(1)).await?;
    let trapdoor_swings = count_swings_until_break_effect(bot2, swinger_id, marker_pos(1)).await?;

    // Clicking the top face of the diamond block places stone above it.
    player1
        .inventory
        .set_stack_in_hand(Hand::Right, ItemStack::new(1, &Item::STONE));
    bot1.use_block(place_target, 7).await?;
    bot1.start_digging(marker_pos(2), 8).await?;
    tick_until_broken(server, marker_pos(2)).await?;
    let placement_swings = count_swings_until_break_effect(bot2, swinger_id, marker_pos(2)).await?;

    player1
        .inventory
        .set_stack_in_hand(Hand::Right, ItemStack::EMPTY.clone());

    if trapdoor_swings != 1 {
        return Err(format!(
            "Expected TestBot2 to get 1 swing after TestBot1 used a trapdoor, got {trapdoor_swings}"
        ));
    }
    if world.get_block(&place_target.up()).id != Block::STONE.id {
        return Err("TestBot1 did not place stone on top of the diamond block".to_string());
    }
    if placement_swings != 1 {
        return Err(format!(
            "Expected TestBot2 to get 1 swing after TestBot1 placed a block, got {placement_swings}"
        ));
    }

    summary_lines.push(format!(
        "[PASS] bot:use_block_swing_reaches_other_players ({}ms)",
        start.elapsed().as_millis()
    ));
    Ok(())
}

async fn tick_until_broken(server: &Arc<Server>, pos: BlockPos) -> Result<(), String> {
    let world = server.get_world_from_dimension(&Dimension::OVERWORLD);
    for _ in 0..MAX_POLL_TICKS {
        tokio::time::sleep(Duration::from_millis(5)).await;
        server.tick();
        if world.get_block(&pos).id == Block::AIR.id {
            return Ok(());
        }
    }
    Err("TestBot1 did not break the marker block in creative".to_string())
}

/// Counts the swings of `entity_id` that `bot` reads before the level event at `pos`.
async fn count_swings_until_break_effect(
    bot: &mut HeadlessBot,
    entity_id: i32,
    pos: BlockPos,
) -> Result<usize, String> {
    let swing_id = CSwingArm::to_id(CURRENT_MC_VERSION);
    let level_event_id = CWorldEvent::to_id(CURRENT_MC_VERSION);
    let mut swings = 0;
    loop {
        let packet = bot.next_packet().await?;
        let mut read = &packet.payload[..];
        if packet.id == swing_id {
            let swung = read.get_var_int().map_err(|err| err.to_string())?;
            if swung.0 == entity_id {
                swings += 1;
            }
        } else if packet.id == level_event_id {
            read.get_i32_be().map_err(|err| err.to_string())?;
            let location = read
                .get_block_pos(&CURRENT_MC_VERSION)
                .map_err(|err| err.to_string())?;
            if location == pos {
                return Ok(swings);
            }
        }
    }
}
