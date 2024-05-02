use regex::Regex;

struct HandlerResult {
    pub count_all_nodes: i32,
    pub count_accepted_nodes: i32,
}

impl HandlerResult {
    pub fn new() -> Self {
        Self { count_all_nodes: 0, count_accepted_nodes: 0 }
    }
}


trait Handler {
    fn handle(&mut self, value: String);
    fn get_result(&mut self, result: HandlerResult) -> HandlerResult;
}

struct Terminator;

impl Terminator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Handler for Terminator {
    fn handle(&mut self, value: String) {
        println!("terminator received {value}")
    }
    fn get_result(&mut self, result: HandlerResult) -> HandlerResult {
        result
    }
}

#[derive(Debug)]
enum CountType {
    ALL,
    ACCEPTED,
}

struct NodesCounter {
    pub count: i32,
    pub count_type: CountType,
    pub next: Box<dyn Handler + 'static>,
}

impl Handler for NodesCounter {
    fn handle(&mut self, value: String) {
        self.count += 1;
        self.next.handle(value)
    }
    fn get_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        match self.count_type {
            CountType::ALL => { result.count_all_nodes = self.count }
            CountType::ACCEPTED => { result.count_accepted_nodes = self.count }
        }
        return self.next.get_result(result);
    }
}

#[derive(Debug)]
enum FilterType {
    AcceptMatching,
    RemoveMatching,
}
struct NodesFilter {
    pub regex: Regex,
    pub filter_type: FilterType,
    pub next: Box<dyn Handler + 'static>,
}

#[derive(Debug)]
enum HandlerDef {
    NodesFilter(NodesFilterDef),
    NodesCounter(NodesCounterDef),
}
#[derive(Debug)]
struct NodesFilterDef{ regex: Regex, filter_type: FilterType }
#[derive(Debug)]
struct NodesCounterDef{ count_type: CountType }

fn as_chain(mut defs: Vec<HandlerDef>) -> Box<dyn Handler> {
    defs.reverse();
    let mut previous: Box<dyn Handler> = Box::new(Terminator::new());
    for hander_def in defs {
        match hander_def {
            HandlerDef::NodesFilter(def) => {
                // println!("regex: {:?}, filter_type: {:?}", &def.regex, &def.filter_type);
                previous = Box::new(NodesFilter{filter_type: def.filter_type, regex: def.regex, next: previous });
            }
            HandlerDef::NodesCounter(def) => {
                // println!("count_type: {:?}", &def.count_type);
                previous = Box::new(NodesCounter{count: 0, count_type: def.count_type, next: previous });
            }
        }
    }
    previous
}

impl Handler for NodesFilter {
    fn handle(&mut self, value: String) {
        match self.filter_type {
            FilterType::AcceptMatching => {
                if self.regex.is_match(&value) {
                    self.next.handle(value)
                }
            }
            FilterType::RemoveMatching => {
                if !self.regex.is_match(&value) {
                    self.next.handle(value)
                }
            }
        }
    }
    fn get_result(&mut self, result: HandlerResult) -> HandlerResult {
        return self.next.get_result(result);
    }
}



#[cfg(test)]
mod tests {
    use regex::Regex;
    use crate::handler::{as_chain, CountType, FilterType, Handler, HandlerDef, HandlerResult, NodesCounter, NodesCounterDef, NodesFilter, NodesFilterDef, Terminator};

    #[test]
    fn test_as_chain() {
        let handlers = vec![
            HandlerDef::NodesCounter(NodesCounterDef{count_type: CountType::ALL}),
            HandlerDef::NodesFilter(NodesFilterDef{regex: Regex::new(".*p.*").unwrap(), filter_type: FilterType::AcceptMatching }),
            HandlerDef::NodesFilter(NodesFilterDef{regex: Regex::new(".*z.*").unwrap(), filter_type: FilterType::RemoveMatching }),
            HandlerDef::NodesCounter(NodesCounterDef{count_type: CountType::ACCEPTED}),
        ];
        let mut handler = as_chain(handlers);

        handler.handle("kasper".to_string());
        handler.handle("seppl".to_string());
        handler.handle("hotzenplotz".to_string());
        handler.handle("großmutter".to_string());
        let result = handler.get_result(HandlerResult::new());
        assert_eq!(result.count_all_nodes, 4);
        assert_eq!(result.count_accepted_nodes, 2);
    }

    #[test]
    fn test_get_field() {
        let mut handler = NodesCounter {
            count: 0,
            count_type: CountType::ALL,
            next: Box::new(NodesFilter {
                filter_type: FilterType::AcceptMatching,
                regex: Regex::new(".*p.*").unwrap(),
                next: Box::new(NodesFilter {
                    filter_type: FilterType::RemoveMatching,
                    regex: Regex::new(".*z.*").unwrap(),
                    next: Box::new(NodesCounter {
                        count: 0,
                        count_type: CountType::ACCEPTED,
                        next: Box::new(Terminator::new()),
                    }),
                }),
            }),
        };

        handler.handle("kasper".to_string());
        handler.handle("seppl".to_string());
        handler.handle("hotzenplotz".to_string());
        handler.handle("großmutter".to_string());
        let result = handler.get_result(HandlerResult::new());
        assert_eq!(result.count_all_nodes, 4);
        assert_eq!(result.count_accepted_nodes, 2);
    }
}