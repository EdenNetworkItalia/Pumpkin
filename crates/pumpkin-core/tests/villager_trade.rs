//! A villager trade through the slot click of a joined player.
#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test fails on the first missing value"
)]

use std::{
    borrow::Cow,
    net::{Ipv4Addr, SocketAddr},
    sync::{Arc, atomic::Ordering, mpsc},
    time::Duration,
};

use arc_swap::ArcSwap;
use pumpkin_config::{AdvancedConfiguration, BasicConfiguration, TelemetryConfig};
use pumpkin_core::{
    data::VanillaData,
    entity::{
        Entity, EntityBase,
        passive::villager::{GossipType, VillagerEntity},
        player::Player,
    },
    net::{
        ClientPlatform, GameProfile, PacketRateLimiter, PlayerConfig,
        java::{JavaClient, pending::PendingConnection},
    },
    server::Server,
};
use pumpkin_data::{entity::EntityType, item::Item, item_stack::ItemStack};
use pumpkin_inventory::{
    inventory::Inventory, merchant::merchant_screen_handler::MerchantScreenHandler,
};
use pumpkin_protocol::{
    codec::{
        item_stack_seralizer::{ItemStackSerializer, OptionalItemStackHash},
        var_int::VarInt,
    },
    java::{
        client::play::MerchantOffer,
        server::play::{SClickSlot, SlotActionType},
    },
};
use pumpkin_util::GameMode;

async fn join(server: &Arc<Server>, name: &str) -> Arc<Player> {
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let address: SocketAddr = listener.local_addr().unwrap();
    let (stream, _) = tokio::join!(tokio::net::TcpStream::connect(address), async {
        listener.accept().await.unwrap()
    });
    let pending = PendingConnection::new(
        stream.unwrap(),
        address,
        0,
        PacketRateLimiter::new(false, 0.0, 0.0),
        Arc::downgrade(server),
    );
    let profile = GameProfile {
        id: uuid::Uuid::new_v4(),
        name: name.to_owned(),
        properties: ArcSwap::from_pointee(Vec::new()),
        profile_actions: None,
    };
    let client = JavaClient::from_pending(pending, profile.clone(), PlayerConfig::default());
    let world = server.worlds.load()[0].clone();
    let player = Arc::new(Player::new(
        Arc::new(ClientPlatform::Java(client)),
        profile,
        PlayerConfig::default(),
        &world,
        GameMode::Survival,
    ));
    world.add_player(&player).unwrap();
    player
}

fn merchant<R>(player: &Player, read: impl FnOnce(&mut MerchantScreenHandler) -> R) -> R {
    let screen = player.current_screen_handler.lock().unwrap().clone();
    let mut screen = screen.lock().unwrap();
    read(
        screen
            .as_any_mut()
            .downcast_mut::<MerchantScreenHandler>()
            .unwrap(),
    )
}

#[test]
fn villager_trade_click_completes_and_the_next_tick_resends_the_offers() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    // `VanillaData::load` reads and writes its files in the working directory.
    std::env::set_current_dir(dir.path()).unwrap();
    let mut advanced = AdvancedConfiguration::default();
    advanced.networking.bedrock.enabled = false;
    let basic = BasicConfiguration {
        default_level_name: dir.path().join("world").to_string_lossy().into_owned(),
        allow_nether: false,
        allow_end: false,
        ..BasicConfiguration::default()
    };
    let server = runtime.block_on(async {
        Server::new(
            basic,
            advanced,
            TelemetryConfig::default(),
            VanillaData::load(),
            Vec::new(),
        )
        .await
        .unwrap()
    });
    let player = runtime.block_on(join(&server, "Trader"));

    let world = player.world();
    let villager = VillagerEntity::new(Entity::new(
        world.clone(),
        player.living_entity.entity.pos.load(),
        &EntityType::VILLAGER,
    ));
    world.spawn_entity(villager.clone());
    villager.offers.lock().unwrap().push(MerchantOffer {
        base_cost_a: ItemStackSerializer(Cow::Owned(ItemStack::new(1, &Item::EMERALD))),
        output: ItemStackSerializer(Cow::Owned(ItemStack::new(1, &Item::BREAD))),
        cost_b: None,
        reward_exp: false,
        uses: 0,
        max_uses: 12,
        xp: 1,
        special_price: 0,
        // High enough that the 2 points of trading gossip give a discount of 1.
        price_multiplier: 0.5,
        demand: 0,
    });
    let sync_id = player.open_handled_screen(villager.as_ref(), None).unwrap();
    villager
        .merchant_inventory
        .set_stack(0, ItemStack::new(1, &Item::EMERALD));
    merchant(&player, MerchantScreenHandler::update_result_slot);

    let packet = SClickSlot {
        sync_id: VarInt(i32::from(sync_id)),
        revision: VarInt(0),
        slot: 2,
        button: SClickSlot::BUTTON_LEFT,
        mode: SlotActionType::Pickup,
        length_of_array: VarInt(0),
        array_of_changed_slots: Vec::new(),
        carried_item: OptionalItemStackHash(None),
    };
    let (done, finished) = mpsc::channel();
    let clicker = player.clone();
    std::thread::spawn(move || {
        clicker.on_slot_click(packet, &server);
        let _ = done.send(());
    });
    finished
        .recv_timeout(Duration::from_secs(30))
        .unwrap_or_else(|_| panic!("the trade click did not finish"));
    assert_eq!(villager.offers.lock().unwrap()[0].uses, 1);
    assert_eq!(
        merchant(&player, |handler| handler.offers[0].special_price),
        0
    );

    // An age that is not a multiple of 20 skips the job site and bed searches of the tick.
    villager.get_entity().age.store(1, Ordering::Relaxed);
    villager.villager_mob_tick();

    let trading = villager.gossips.lock().unwrap()[&player.gameprofile.id][&GossipType::Trading];
    assert_eq!(trading, 2);
    assert_eq!(
        merchant(&player, |handler| handler.offers[0].special_price),
        -1
    );
}
