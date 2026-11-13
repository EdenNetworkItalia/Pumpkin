use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use pumpkin_core::server::Server;
use pumpkin_data::{dimension::Dimension, packet::CURRENT_MC_VERSION};
use pumpkin_protocol::{java::client::play::CSystemChatMessage, packet::MultiVersionJavaPacket};
use pumpkin_util::PermissionLvl;

use crate::bot::{HeadlessBot, MAX_POLL_TICKS};

const JOINED_KEY: &str = "multiplayer.player.joined";
const SEED_KEY: &str = "commands.seed.success";

/// Vanilla `PlayerList.placeNewPlayer` broadcasts the join message before it adds the joiner to
/// `players`: the other players get it and the joiner does not.
pub async fn run_join_message_test(
    server: &Arc<Server>,
    addr: SocketAddr,
    bot1: &mut HeadlessBot,
    summary_lines: &mut Vec<String>,
) -> Result<(), String> {
    let start = Instant::now();
    let world = server.get_world_from_dimension(&Dimension::OVERWORLD);

    let ticker = {
        let server = server.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(5)).await;
                server.tick();
            }
        })
    };
    let joiner_got = async {
        let mut joiner = HeadlessBot::connect_and_join(addr, "TestBot3").await?;
        let Some(player3) = world.get_player_by_name("TestBot3") else {
            return Err("Could not find 'TestBot3' in Overworld".to_string());
        };
        let dispatcher = server.command_dispatcher.load();
        player3.set_permission_lvl(server, PermissionLvl::Four, &dispatcher);

        // The server queues the join message for every recipient in one call, so the message
        // that TestBot1 receives is the one that TestBot3 would have received too.
        read_system_chats(bot1, &[JOINED_KEY, "TestBot3"]).await?;

        // The feedback of /seed comes after the join of TestBot3 is complete.
        joiner.send_command("seed").await?;
        let joined = read_system_chats(&mut joiner, &[SEED_KEY]).await?;
        Ok::<_, String>(joined)
    }
    .await;

    for _ in 0..MAX_POLL_TICKS {
        if server.get_player_count() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    ticker.abort();

    let joined = joiner_got?;
    if joined != 0 {
        return Err(format!(
            "Expected TestBot3 to get no join message of its own, got {joined}"
        ));
    }

    summary_lines.push(format!(
        "[PASS] bot:join_message_skips_joiner ({}ms)",
        start.elapsed().as_millis()
    ));
    Ok(())
}

/// Reads packets of `bot` until a system chat message holds every text of `until`. Returns how
/// many earlier system chat messages were join messages for `TestBot3`.
async fn read_system_chats(bot: &mut HeadlessBot, until: &[&str]) -> Result<usize, String> {
    let system_chat_id = CSystemChatMessage::to_id(CURRENT_MC_VERSION);
    let mut own_joins = 0;
    loop {
        let packet = bot.next_packet().await?;
        if packet.id != system_chat_id {
            continue;
        }
        if contains_all(&packet.payload, until) {
            return Ok(own_joins);
        }
        if contains_all(&packet.payload, &[JOINED_KEY, "TestBot3"]) {
            own_joins += 1;
        }
    }
}

fn contains_all(payload: &[u8], texts: &[&str]) -> bool {
    texts.iter().all(|text| {
        payload
            .windows(text.len())
            .any(|window| window == text.as_bytes())
    })
}
