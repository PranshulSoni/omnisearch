#[path = "../markdown.rs"]
mod markdown;

fn main() {
    let prompt = "can you execute any cmmand for me\n---\nUser: please execute\n---\nUser: run a command to run for me that lists all folders in documents";
    let response = "\n\n---\n\nSure, I can execute commands for you. What command would you like me to run?\n\n---\n\nThe command is pending your approval. Here's what it will do:\n\n- List all **folders** (not files) inside your `Documents` directory\n- Show them sorted alphabetically\n- Print the total count\n\nPlease approve the execution so I can show you the results!";
    
    println!("Parsing prompt...");
    let parsed_prompt = markdown::parse(prompt);
    println!("Parsed prompt: {:?}", parsed_prompt);
    
    println!("Parsing response...");
    let parsed_response = markdown::parse(response);
    println!("Parsed response: {:?}", parsed_response);
    
    println!("Format conversation...");
    let conversation = format_conversation(prompt, response);
    println!("Conversation formatted: {}", conversation);

    println!("Parsing formatted conversation...");
    let parsed_conv = markdown::parse(&conversation);
    println!("Parsed formatted: {} blocks", parsed_conv.len());
    
    println!("Testing split rendering...");
    let parts: Vec<&str> = conversation.split("\n\n---\n\n").collect();
    for part in &parts {
        let mut prompt = "";
        let mut response = "";
        if part.starts_with("User: ") {
            let after_user = &part["User: ".len()..];
            if let Some((p, r)) = after_user.split_once("\n\n") {
                prompt = p.trim();
                response = r.trim();
            } else {
                prompt = after_user.trim();
            }
        } else {
            response = part.trim();
        }
        println!("Part - Prompt: {:?}, Response: {:?}", prompt, response);
        if !response.is_empty() {
            let blocks = markdown::parse(response);
            println!("Parsed part response: {} blocks", blocks.len());
        }
    }
    println!("All markdown parsing checks passed!");
}

fn format_conversation(prompt: &str, response: &str) -> String {
    let prompts: Vec<&str> = prompt.split("\n---\n").map(|p| {
        p.strip_prefix("User: ").unwrap_or(p).trim()
    }).collect();
    let responses: Vec<&str> = response.split("\n\n---\n\n").collect();

    let mut conversation = String::new();
    for i in 0..prompts.len() {
        if i > 0 {
            conversation.push_str("\n\n---\n\n");
        }
        let p = prompts[i];
        if !p.is_empty() {
            conversation.push_str("User: ");
            conversation.push_str(p);
            conversation.push_str("\n\n");
        }
        if i < responses.len() {
            let r = responses[i].trim();
            if !r.is_empty() {
                conversation.push_str(r);
            }
        }
    }
    if responses.len() > prompts.len() {
        for i in prompts.len()..responses.len() {
            if i > 0 {
                conversation.push_str("\n\n---\n\n");
            }
            conversation.push_str(responses[i].trim());
        }
    }
    conversation
}
