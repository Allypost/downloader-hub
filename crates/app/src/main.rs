#[tokio::main]
#[allow(unreachable_code)]
async fn main() {
    #[cfg(all(feature = "server", feature = "cli", not(debug_assertions)))]
    {
        return compile_error!("Can't use both `cli` and `server` features at the same time");
    }

    #[cfg(all(not(feature = "server"), not(feature = "cli")))]
    {
        return compile_error!("Must select either `cli` or `server` feature");
    }

    #[cfg(feature = "cli")]
    {
        return app_cli::run().await;
    }

    #[cfg(feature = "server")]
    {
        return app_server::run().await;
    }
}
