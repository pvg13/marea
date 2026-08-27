use std::collections::BTreeSet;

use marea_cli::manifest::{self, AppCrate, Ctx, MareaSource};
use marea_cli::options::{Auth, Engine, Options, Target};

#[test]
#[ignore = "inspection helper"]
fn dump() {
    let o = Options {
        name: "my-app".into(),
        targets: [Target::Android, Target::Web]
            .into_iter()
            .collect::<BTreeSet<_>>(),
        auth: Some(Auth {
            pocketbase_url: "https://admin.almares.es".into(),
        }),
        engine: Engine::WaveSync {
            relay: Some("/dns4/relay.example/udp/4011/quic-v1/p2p/12D3Koo".into()),
        },
        ..Default::default()
    };
    let mut o = o;
    o.features.pairing = true;
    o.features.android_back = true;
    let ctx = Ctx::new(
        &o,
        MareaSource::Path {
            prefix: "../marea".into(),
        },
    );
    println!(
        "======== workspace Cargo.toml ========\n{}",
        manifest::workspace_manifest(&ctx)
    );
    println!(
        "======== crates/data/Cargo.toml ========\n{}",
        manifest::data_manifest(&ctx)
    );
    println!(
        "======== crates/app-mobile/Cargo.toml ========\n{}",
        manifest::app_manifest(&ctx, AppCrate::Mobile)
    );
}
