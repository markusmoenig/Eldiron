//! Account setup is a host command, never an in-game mode or tool.
use shared::ai::chatgpt::Manager;
use std::sync::atomic::AtomicBool;
pub(super) fn run(args: &[String]) -> Result<(), String> {
    let manager = Manager::local()?;
    match args.first().map(String::as_str) {
        Some("login") if args.len()==1 || (args.len()==2 && args[1]=="--new")=>{
            println!("Continue with ChatGPT in your browser. Waiting up to five minutes…");
            let account=manager.sign_in(args.len()==2,&AtomicBool::new(false))?;
            println!("Signed in: {}",account.label);
            if !account.plan_enabled {println!("ChatGPT plan usage was not authorized. Sign in again to enable it.");return Ok(());}
            let models=match manager.models() {
                Ok(models)=>models,
                Err(error)=>return Err(format!("{error} Sign-in was saved. Retry with: eldiron-client-terminal auth models")),
            };
            for model in models {println!("{}  {}",model.slug,model.display_name);}
            println!("Choose a model with: eldiron-client-terminal auth model MODEL_SLUG");
        },
        Some("logout") if args.len()==1=>println!("{}",manager.sign_out()?),
        Some("status"|"accounts") if args.len()==1=>{
            let selected=manager.selected()?.map(|a|a.id);
            for account in manager.accounts()? {
                println!("{} {}  {}  {}  model={}",if Some(account.id)==selected {"*"}else{" "},account.id,account.label,
                    if !account.signed_in {"signed out"}else if account.plan_enabled {"ChatGPT plan authorized"}else{"plan usage not authorized"}, account.model.as_deref().unwrap_or("not selected"));
            }
            if selected.is_none(){println!("Not signed in. Run: eldiron-client-terminal auth login");}
        },
        Some("use") if args.len()==2=>{manager.select(args[1].parse().map_err(|_|"Expected a saved ChatGPT account UUID.")?)?;println!("ChatGPT account selected.");},
        Some("models") if args.len()==1=>{for model in manager.models()? {println!("{}  {}",model.slug,model.display_name);}},
        Some("model") if args.len()==2=>{
            let models=manager.models()?;
            if !models.iter().any(|m|m.slug==args[1]) {return Err("Model is not in the selected account's catalog. Run auth models.".into());}
            manager.set_model(&args[1])?;println!("ChatGPT model selected.");
        },
        _=>return Err("Usage: eldiron-client-terminal auth login [--new] | logout | status | accounts | use ACCOUNT_UUID | models | model MODEL_SLUG".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::*;
    #[test]
    fn chatgpt_terminal_provider_selection_uses_existing_text_presentation() {
        let args = [
            "eldiron-client-terminal",
            "adventure.eldiron",
            "--mode",
            "text",
            "--ai-provider",
            "chatgpt",
        ]
        .map(str::to_string);
        let options = parse_terminal_args(&args).unwrap();
        assert!(options.mode == Some(TerminalPlayMode::Text));
        assert_eq!(options.ai_provider.as_deref(), Some("chatgpt"));
        assert_eq!(options.path, Some(PathBuf::from("adventure.eldiron")));
        assert!(
            parse_terminal_args(&["client", "--ai-provider", "invalid"].map(str::to_string))
                .is_err()
        );
        assert!(parse_terminal_args(&["client", "--ai-provider"].map(str::to_string)).is_err());
    }
}
