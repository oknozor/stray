use stray::StatusNotifierWatcher;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> stray::error::Result<()> {
    let (_, cmd_rx) = mpsc::channel(10);
    let tray = StatusNotifierWatcher::new(cmd_rx).await?;

    let mut host_one = tray.create_notifier_host("host_one").await.unwrap();

    while let Ok(mesage) = host_one.recv().await {
        println!("Message from host one {:?}", mesage);
    }

    Ok(())
}
