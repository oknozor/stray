use stray::StatusNotifierWatcher;
use tokio::join;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> stray::error::Result<()> {
    let (_cmd_tx, cmd_rx) = mpsc::channel(10);
    let tray = StatusNotifierWatcher::new(cmd_rx).await?;

    let mut host_one = tray.create_notifier_host("host_one").await.unwrap();
    let mut host_two = tray.create_notifier_host("host_two").await.unwrap();

    let one = tokio::spawn(async move {
        while let Ok(mesage) = host_one.recv().await {
            println!("Message from host one {:?}", mesage);
        }
    });

    let two = tokio::spawn(async move {
        let mut count = 0;
        while let Ok(mesage) = host_two.recv().await {
            count += 1;
            if count > 5 {
                break;
            }
            println!("Message from host two {:?}", mesage);
        }

        host_two.destroy().await?;
        stray::error::Result::<()>::Ok(())
    });

    let _ = join!(one, two);
    Ok(())
}
