use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::{sync::RwLock, time::sleep};
use tracing::{instrument, Level};

use crate::controller::handler::process_ctx_results;
use crate::controller::CtxResult;
use crate::controller::DialCtxActions;

use super::BotAdapter;

#[instrument(level = Level::DEBUG, skip(dial_ctx, bot_adapter))]
pub async fn track_dialog_ttl<B: BotAdapter, C: DialCtxActions>(
    dial_ctx: Arc<RwLock<C>>,
    bot_adapter: Arc<B>,
    max_ttl_seconds: u64,
    close_msg: Option<String>,
) {
    loop {
        let current_time = SystemTime::now();

        let result = dial_ctx
            .read()
            .await
            .dialogues_list()
            .into_iter()
            .map(|(user_id, controller)| {
                (
                    *user_id,
                    current_time
                        .duration_since(*controller.get_last_interaction_time())
                        .unwrap(),
                )
            })
            .collect::<Vec<(u64, Duration)>>();

        let sleep_time = result
            .iter()
            .filter_map(|(_, duration)| {
                if duration.as_secs().le(&max_ttl_seconds) {
                    Some(Duration::from_secs(max_ttl_seconds - duration.as_secs()))
                } else {
                    None
                }
            })
            .max()
            .unwrap_or_else(|| Duration::from_secs(max_ttl_seconds));

        let keys_to_remove = result
            .iter()
            .filter_map(|(user_id, duration)| {
                duration.as_secs().ge(&max_ttl_seconds).then_some(user_id)
            })
            .collect::<Vec<&u64>>();

        if !keys_to_remove.is_empty() {
            tracing::debug!("Remove {} dialogs", keys_to_remove.len());
            let mut context_wlock = dial_ctx.write().await;

            let controllers_iter = keys_to_remove.into_iter().filter_map(|user_id| {
                context_wlock
                    .take_controller(user_id)
                    .map(|controller| (*user_id, controller))
            });

            let mut result: Vec<(u64, Vec<CtxResult>)> = Vec::new();
            for (user_id, controller) in controllers_iter {
                match controller.shutdown().await {
                    Ok(ctx_result) => result.push((user_id, ctx_result)),
                    Err(err) => {
                        tracing::error!("Failed dialog shutdown {}", err);
                    }
                }
            }
            drop(context_wlock);

            for (user_id, ctx_results) in result {
                if let Err(err) = process_ctx_results(user_id, ctx_results, &bot_adapter).await {
                    tracing::error!("Failed results processing for {}: {}", user_id, err);
                } else if let Some(msg) = close_msg.clone() {
                    if let Err(err) = bot_adapter.send_message(user_id, msg.into()).await {
                        tracing::error!("Failed send message for user {}: {}", user_id, err,)
                    }
                }
            }
        }

        tracing::debug!("Sleep {} seconds", sleep_time.as_secs());
        sleep(sleep_time).await;
    }
}
