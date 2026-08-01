mod resolution;
mod roll;
mod runtime;
mod types;

pub use runtime::GameSession;
pub use types::*;

pub(crate) fn generated_session_typescript() -> String {
    use ts_rs::TS;

    let declarations = [
        TurnSide::decl(),
        SessionOutcome::decl(),
        PartyMemberSelectionPolicy::decl(),
        PartySquareTargetReceipt::decl(),
        ActivationView::decl(),
        TurnReceipt::decl(),
        SessionView::decl(),
    ]
    .into_iter()
    .map(|declaration| format!("export {declaration}"))
    .collect::<Vec<_>>()
    .join("\n\n");
    format!(
        "export const SESSION_VIEW_SCHEMA_VERSION = {SESSION_VIEW_SCHEMA_VERSION} as const;\n\
export const SESSION_VIEW_LIMITS = Object.freeze({{\n\
  maxActivations: {MAX_SESSION_ACTIVATIONS},\n\
  maxReceipts: {MAX_SESSION_RECEIPTS},\n\
}} as const);\n\n\
{declarations}"
    )
}

#[cfg(test)]
mod tests;
