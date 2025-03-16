
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
    Plain(String),
}

#[derive(Debug, Clone)]
pub struct EventProcessor;

impl EventProcessor {
    pub async fn process(&self, event: Event) -> Response {
        match event {
            Event::ChatInput(chat_input) => {
                Response::Plain(format!("you said: {}", chat_input.content))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let event: ChatInput = "hello".into();
        let event: Event = Event::ChatInput(event);
        let processor = EventProcessor;
        let response = processor.process(event).await;
        assert_eq!(response, Response::Plain("you said: hello".to_string()));
    }
}
