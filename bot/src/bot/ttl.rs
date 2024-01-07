use std::time::{Duration, SystemTime};

use sec_store::repository::RecordsRepository;
use teloxide::{requests::Requester, types::UserId, Bot};
use tokio::{sync::RwLock, time::sleep};

use crate::dialogues_controller::handler::process_ctx_results;
use crate::dialogues_controller::DialCtxActions;
use crate::{dialogues_controller::CtxResult, user_repo_factory::RepositoriesFactory};
use std::sync::Arc;

use super::BotContext;

const DIALOG_CONTROLLER_TTL_SECONDS: u64 = 300;
pub async fn track_dialog_ttl<F: RepositoriesFactory<R>, R: RecordsRepository>(
    bot_context: Arc<RwLock<BotContext<F, R>>>,
    bot: Bot,
) {
    loop {
        let current_time = SystemTime::now();

        let result = bot_context
            .read()
            .await
            .dial
            .read()
            .await
            .dial_ctxs
            .iter()
            .map(|(user_id, controller)| {
                (
                    *user_id,
                    current_time
                        .duration_since(*controller.get_last_interaction_time())
                        .unwrap(),
                )
            })
            .collect::<Vec<(UserId, Duration)>>();

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
            .collect::<Vec<&UserId>>();

        if !keys_to_remove.is_empty() {
            log::debug!("[ttl controller] Remove {} dialogs", keys_to_remove.len());
            let context_wlock = bot_context.write().await;

            let result = keys_to_remove
                .into_iter()
                .filter_map(|user_id| {
                    context_wlock
                        .dial
                        .try_write()
                        .unwrap()
                        .take_controller(&user_id.0)
                        .map(|controller| (*user_id, controller))
                })
                .filter_map(|(user_id, controller)| match controller.shutdown() {
                    Ok(result) => Some((user_id, result)),
                    Err(err) => {
                        log::error!("[ttl controller] Failed dialog shutdown {}", err);
                        None
                    }
                })
                .collect::<Vec<(UserId, Vec<CtxResult>)>>();

            let context_rlock = context_wlock.downgrade();
            for (user_id, ctx_results) in result {
                if let Err(err) = process_ctx_results(user_id.0, ctx_results, &context_rlock.bot_adapter).await {
                    log::error!(
                        "[ttl controller] Failed results processing for {}: {}",
                        user_id,
                        err
                    );
                } else if let Err(err) = bot
                    .send_message(
                        user_id,
                        format!(
                            "Диалог закрыт потому что не использовался {} секунд 🙈\nВведите /start чтобы инициировать новый диалог",
                            DIALOG_CONTROLLER_TTL_SECONDS
                        ),
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
