use std::collections::HashMap;

use lazy_static::lazy_static;

const GNU_PLUS_LINUX: &'static str = "I'd just like to interject for a moment. What you're referring to as Linux, is in fact, GNU/Linux, or as I've recently taken to calling it, GNU plus Linux. Linux is not an operating system unto itself, but rather another free component of a fully functioning GNU system made useful by the GNU corelibs, shell utilities and vital system components comprising a full OS as defined by POSIX.

Many computer users run a modified version of the GNU system every day, without realizing it. Through a peculiar turn of events, the version of GNU which is widely used today is often called \"Linux\", and many of its users are not aware that it is basically the GNU system, developed by the GNU Project.

There really is a Linux, and these people are using it, but it is just a part of the system they use. Linux is the kernel: the program in the system that allocates the machine's resources to the other programs that you run. The kernel is an essential part of an operating system, but useless by itself; it can only function in the context of a complete operating system. Linux is normally used in combination with the GNU operating system: the whole system is basically GNU with Linux added, or GNU/Linux. All the so-called \"Linux\" distributions are really distributions of GNU/Linux.
";

const GOOGLERS: &'static str = "The key point here is our programmers are Googlers, they’re not researchers. They’re typically, fairly young, fresh out of school, probably learned Java, maybe learned C or C++, probably learned Python. They’re not capable of understanding a brilliant language but we want to use them to build good software. So, the language that we give them has to be easy for them to understand and easy to adopt.";

const RUST: &'static str = "Rust has zero-cost abstractions, move semantics, guaranteed memory safety, threads without data races, trait-based generics, pattern matching, type inference, minimal runtime and efficient C bindings.";

const RICK_AND_MORTY: &'static str = "To be fair, you have to have a very high IQ to understand Rick and Morty. The humour is extremely subtle, and without a solid grasp of theoretical physics most of the jokes will go over a typical viewer's head. There's also Rick's nihilistic outlook, which is deftly woven into his characterisation- his personal philosophy draws heavily from Narodnaya Volya literature, for instance. The fans understand this stuff; they have the intellectual capacity to truly appreciate the depths of these jokes, to realise that they're not just funny- they say something deep about LIFE. As a consequence people who dislike Rick & Morty truly ARE idiots- of course they wouldn't appreciate, for instance, the humour in Rick's existential catchphrase \"Wubba Lubba Dub Dub,\" which itself is a cryptic reference to Turgenev's Russian epic Fathers and Sons. I'm smirking right now just imagining one of those addlepated simpletons scratching their heads in confusion as Dan Harmon's genius wit unfolds itself on their television screens. What fools.. how I pity them. 😂";

pub const OPEN_SOURCE_MAINTAINERS: &'static str = "Most maintainers start working on open source software because it’s fun and solves a problem they have. Many continue out of a sense of obligation instead of fun and over time this unpaid, increasingly non-fun work grinds them down. When they make a controversial decision and receive abuse for it, their friends and family start to ask them if open source is worth the grief.";

<<<<<<< HEAD
const TBF_MR_ROBOT: &'static str = "To be fair, you have to have a very high IQ to understand Mr. Robot. The plot is extremely subtle, and without a solid grasp of psychology most of the twists will go over a typical viewer's head. There's also Mr. Robot's nihilistic outlook, which is deftly woven into his characterisation- his personal philosophy draws heavily from anarchist literature, for instance. The fans understand this stuff; they have the intellectual capacity to truly appreciate the depths of these stories, to realise that they're not just cool- they say something deep about LIFE. As a consequence people who dislike Mr. Robot truly ARE idiots- of course they wouldn't appreciate, for instance, the irony in Elliot's existential dialogue with the viewer - \"Hello, friend\" which itself is a cryptic reference to the psychological phenomenon known as parasocial obsession. I'm smirking right now just imagining one of those addlepated simpletons scratching their heads in confusion as Sam Esmail's genius wit unfolds itself on their television screens. What fools.. how I pity them. 😂
=======
pub const TBF_MR_ROBOT: &'static str = "To be fair, you have to have a very high IQ to understand Mr. Robot. The plot is extremely subtle, and without a solid grasp of psychology most of the twists will go over a typical viewer's head. There's also Mr. Robot's nihilistic outlook, which is deftly woven into his characterisation- his personal philosophy draws heavily from anarchist literature, for instance. The fans understand this stuff; they have the intellectual capacity to truly appreciate the depths of these stories, to realise that they're not just cool- they say something deep about LIFE. As a consequence people who dislike Mr. Robot truly ARE idiots- of course they wouldn't appreciate, for instance, the irony in Elliot's existential dialogue with the viewer - \"Hello, friend\" which itself is a cryptic reference to the psychological phenomenon known as parasocial obsession. I'm smirking right now just imagining one of those addlepated simpletons scratching their heads in confusion as Sam Esmail's genius wit unfolds itself on their television screens. What fools.. how I pity them. 😂
>>>>>>> 802bb17 (Forgout to escape double quotes)

And yes, by the way, i DO have an Fsociety tattoo. And no, you cannot see it. It's for the ladies' eyes only- and even then they have to demonstrate that they're within 5 IQ points of my own (preferably lower) beforehand. Nothin personnel kid 😎
";

lazy_static! {
    pub static ref COPYPASTAS: HashMap<String, String> = {
	let mut map = HashMap::new();
	map.insert(String::from("linux"), GNU_PLUS_LINUX.to_owned());
	map.insert(String::from("googlers"), GOOGLERS.to_owned());
	map.insert(String::from("rust"), RUST.to_owned());
	map.insert(String::from("rick and morty"), RICK_AND_MORTY.to_owned());
	map.insert(String::from("open source maintainers"), OPEN_SOURCE_MAINTAINERS.to_owned());
	map.insert(String::from("mr. robot"), TBF_MR_ROBOT.to_owned());
	map
    };
}

pub fn get<S: AsRef<str>>(key: S) -> Option<&'static String> {
    COPYPASTAS.get(key.as_ref())
}
