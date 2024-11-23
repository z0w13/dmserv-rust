#[macro_export]
macro_rules! spawn_task {
    ( $interval:expr, $task:expr, $ctx:ident, $data:ident ) => {
        use tokio::spawn;
        use tokio_schedule::{every, Job};
        use tracing::{debug, error};

        {
            let data = $data.to_owned();
            let ctx = $ctx.to_owned();
            spawn(every($interval).seconds().perform(move || {
                let data = data.to_owned();
                let ctx = ctx.to_owned();

                async move {
                    debug!("executing {}", stringify!($task));
                    if let Err(err) = $task(&ctx, data.clone()).await {
                        error!("error executing {}: {}", stringify!($task), err)
                    }
                    debug!("executed {}", stringify!($task));
                }
            }));
        }
    };
}
