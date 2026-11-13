//! The cost of a fire site with no handler, with a handler on another event only, and with one
//! no-op handler. `fire_blocking` is a site that builds the event first, `fire_blocking_with` a
//! site that builds it only when a handler is registered. `InventoryMoveItemEvent` allocates its
//! item id, `EntityAirChangeEvent` holds only integers.

#[path = "../tests/support/mod.rs"]
mod support;

use std::{hint::black_box, sync::Arc};

use criterion::{Criterion, criterion_group, criterion_main};
use pumpkin_core::{
    plugin::{
        BoxFuture, EventHandler, EventPriority, Payload, PluginManager,
        entity::entity_air_change::EntityAirChangeEvent,
        inventory::inventory_move_item::InventoryMoveItemEvent, world::chunk_load::ChunkLoad,
    },
    server::Server,
};
use pumpkin_util::math::position::BlockPos;

struct NoOp;

impl EventHandler<InventoryMoveItemEvent> for NoOp {
    fn handle_blocking<'a>(
        &'a self,
        _server: &'a Arc<Server>,
        _event: &'a mut InventoryMoveItemEvent,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }
}

impl EventHandler<EntityAirChangeEvent> for NoOp {
    fn handle_blocking<'a>(
        &'a self,
        _server: &'a Arc<Server>,
        _event: &'a mut EntityAirChangeEvent,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }
}

impl EventHandler<ChunkLoad> for NoOp {}

fn bench_event<E>(c: &mut Criterion, name: &str, make: fn() -> E)
where
    E: Payload + Send + Sync + 'static,
    NoOp: EventHandler<E>,
{
    let server = support::server();
    let mut group = c.benchmark_group(format!("event_dispatch/{name}"));

    let mut run = |case: &str, manager: &PluginManager| {
        group.bench_function(format!("{case}/fire_blocking"), |b| {
            b.iter(|| {
                let mut event = make();
                manager.fire_blocking(server, &mut event);
                black_box(event);
            });
        });
        group.bench_function(format!("{case}/fire_blocking_with"), |b| {
            b.iter(|| black_box(manager.fire_blocking_with(server, make)));
        });
    };

    let manager = PluginManager::new(Vec::new());
    run("zero_handlers", &manager);

    manager.register::<ChunkLoad, _>(Arc::new(NoOp), EventPriority::Normal, false);
    run("other_event_registered", &manager);

    manager.register::<E, _>(Arc::new(NoOp), EventPriority::Normal, true);
    run("one_handler", &manager);

    group.finish();
}

fn bench_event_dispatch(c: &mut Criterion) {
    bench_event(c, "inventory_move_item", || {
        InventoryMoveItemEvent::new(
            BlockPos::new(0, 64, 0),
            BlockPos::new(1, 64, 0),
            black_box("stone").to_owned(),
            1,
        )
    });
    bench_event(c, "entity_air_change", || {
        EntityAirChangeEvent::new(black_box(1), black_box(300))
    });
}

criterion_group!(benches, bench_event_dispatch);
criterion_main!(benches);
