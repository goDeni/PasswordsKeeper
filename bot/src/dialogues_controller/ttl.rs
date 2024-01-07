use std::time::{Duration, SystemTime};

use tokio::{sync::RwLock, time::sleep};

use crate::dialogues_controller::handler::process_ctx_results;
use crate::dialogues_controller::CtxResult;
use crate::dialogues_controller::DialCtxActions;

use super::BotAdapter;

const DIALOG_CONTROLLER_TTL_SECONDS: u64 = 300;
pub async fn track_dialog_ttl<B: BotAdapter, C: DialCtxActions>(
    dial_ctx: &RwLock<C>,
    bot_adapter: &B,
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
                duration
                    .as_secs()
                    .le(&DIALOG_CONTROLLER_TTL_SECONDS)
                    .then(|| {
                        Some(Duration::from_secs(
                            DIALOG_CONTROLLER_TTL_SECONDS - duration.as_secs(),
                        ))
                    })
                    .unwrap_or(None)
            })
            .max()
            .unwrap_or_else(|| Duration::from_secs(DIALOG_CONTROLLER_TTL_SECONDS));

        let keys_to_remove = result
            .iter()
            .filter_map(|(user_id, duration)| {
                duration
                    .as_secs()
                    .ge(&DIALOG_CONTROLLER_TTL_SECONDS)
                    .then_some(user_id)
            })
            .collect::<Vec<&u64>>();

        if !keys_to_remove.is_empty() {
            log::debug!("[ttl controller] Remove {} dialogs", keys_to_remove.len());
            let mut context_wlock = dial_ctx.write().await;

            let result = keys_to_remove
                .into_iter()
                .filter_map(|user_id| {
                    context_wlock
                        .take_controller(user_id)
                        .map(|controller| (*user_id, controller))
                })
                .filter_map(|(user_id, controller)| match controller.shutdown() {
                    Ok(result) => Some((user_id, result)),
                    Err(err) => {
                        log::error!("[ttl controller] Failed dialog shutdown {}", err);
                        None
                    }
                })
                .collect::<Vec<(u64, Vec<CtxResult>)>>();
            drop(context_wlock);

            for (user_id, ctx_results) in result {
                if let Err(err) = process_ctx_results(user_id, ctx_results, bot_adapter).await {
                    log::error!(
                        "[ttl controller] Failed results processing for {}: {}",
                        user_id,
                        err
                    );
                } else if let Err(err) = bot_adapter.send_message(
                    user_id,
                    format!(
                        "Диалог закрыт потому что не использовался {} секунд 🙈\nВведите /start чтобы инициировать новый диалог",
                        DIALOG_CONTROLLER_TTL_SECONDS
                    ).into()
                )
                    .await
                {
                    log::error!(
                        "[ttl controller] Failed send message for user {}: {}",
                        user_id,
                        err
                    )
                }
            }
        }

        log::debug!("[ttl controller] Sleep {} seconds", sleep_time.as_secs());
        sleep(sleep_time).await;
    }
}
