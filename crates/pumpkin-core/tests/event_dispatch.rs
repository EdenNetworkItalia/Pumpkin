#![allow(clippy::unwrap_used)]

mod support;

use std::sync::Arc;

use pumpkin_core::{
    plugin::{
        BoxFuture, EventHandler, EventPriority, PluginManager,
        entity::entity_air_change::EntityAirChangeEvent,
        inventory::inventory_move_item::InventoryMoveItemEvent,
    },
    server::Server,
};
use pumpkin_util::math::position::BlockPos;

struct CancelAndChange;

impl EventHandler<InventoryMoveItemEvent> for CancelAndChange {
    fn handle_blocking<'a>(
        &'a self,
        _server: &'a Arc<Server>,
        event: &'a mut InventoryMoveItemEvent,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            event.item_amount += 1;
            event.cancelled = true;
        })
    }
}

impl EventHandler<EntityAirChangeEvent> for CancelAndChange {}

fn move_event() -> InventoryMoveItemEvent {
    InventoryMoveItemEvent::new(
        BlockPos::new(0, 64, 0),
        BlockPos::new(1, 64, 0),
        "stone".to_owned(),
        1,
    )
}

#[test]
fn no_handler_skips_the_event() {
    let server = support::server();
    let manager = PluginManager::new(Vec::new());
    let mut built = false;
    let fired = manager.fire_blocking_with(server, || {
        built = true;
        move_event()
    });
    assert!(fired.is_none());
    assert!(!built);

    // A handler of another event makes the map non-empty, so the lookup by name runs.
    manager.register::<EntityAirChangeEvent, _>(
        Arc::new(CancelAndChange),
        EventPriority::Normal,
        true,
    );
    let fired = support::RUNTIME.block_on(manager.fire_with(server, || {
        built = true;
        move_event()
    }));
    assert!(fired.is_none());
    assert!(!built);
}

#[test]
fn handler_sees_the_same_event_as_fire_blocking() {
    let server = support::server();
    let manager = PluginManager::new(Vec::new());
    manager.register::<InventoryMoveItemEvent, _>(
        Arc::new(CancelAndChange),
        EventPriority::Normal,
        true,
    );

    let mut expected = move_event();
    manager.fire_blocking(server, &mut expected);
    let blocking = manager.fire_blocking_with(server, move_event).unwrap();
    let async_fired = support::RUNTIME
        .block_on(manager.fire_with(server, move_event))
        .unwrap();

    for event in [blocking, async_fired] {
        assert_eq!(event.item_amount, expected.item_amount);
        assert_eq!(event.item_id, expected.item_id);
        assert_eq!(event.cancelled, expected.cancelled);
    }
    assert_eq!(expected.item_amount, 2);
    assert!(expected.cancelled);
}
