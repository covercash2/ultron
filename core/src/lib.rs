#[derive(Debug, Clone, PartialEq)]
pub struct ChatInput {
    content: String,
}

impl<T> From<T> for ChatInput
where
    T: ToString,
{
    fn from(content: T) -> Self {
        ChatInput {
            content: content.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    ChatInput(ChatInput),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    PlainChat(String),
}

#[derive(Debug, Clone)]
pub struct EventProcessor;

impl EventProcessor {
    pub async fn process(&self, event: Event) -> Option<Response> {
        tracing::debug!("processing event: {:?}", event);
        match event {
            Event::ChatInput(chat_input) => {
                chat_input
                    .content
                    .starts_with("!ultron")
                    .then_some(Response::PlainChat(format!(
                        "you said: {}",
                        chat_input.content
                    )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let event: ChatInput = "!ultron hello".into();
        let event: Event = Event::ChatInput(event);
        let processor = EventProcessor;
        let response = processor.process(event).await.expect("should get a response");
        assert_eq!(response, Response::PlainChat("you said: hello".to_string()));
    }
}
