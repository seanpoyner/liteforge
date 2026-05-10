//! Intent routing for multi-agent orchestration.
//!
//! Routes user inputs to appropriate agents based on intent classification.

use super::types::{Intent, RoutingDecision};

/// A route definition mapping intents to agents.
#[derive(Debug, Clone)]
pub struct IntentRoute {
    /// Intent patterns to match.
    pub patterns: Vec<String>,
    /// Target agent for this route.
    pub agent: String,
    /// Priority (higher = more important).
    pub priority: i32,
    /// Keywords that boost confidence when present.
    pub keywords: Vec<String>,
}

impl IntentRoute {
    /// Create a new intent route.
    pub fn new(agent: impl Into<String>) -> Self {
        Self {
            patterns: Vec::new(),
            agent: agent.into(),
            priority: 0,
            keywords: Vec::new(),
        }
    }

    /// Add an intent pattern.
    pub fn pattern(mut self, pattern: impl Into<String>) -> Self {
        self.patterns.push(pattern.into());
        self
    }

    /// Add multiple patterns.
    pub fn patterns(mut self, patterns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.patterns.extend(patterns.into_iter().map(|p| p.into()));
        self
    }

    /// Set priority.
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Add a keyword.
    pub fn keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into().to_lowercase());
        self
    }

    /// Add multiple keywords.
    pub fn keywords(mut self, keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keywords
            .extend(keywords.into_iter().map(|k| k.into().to_lowercase()));
        self
    }
}

/// Intent router for directing inputs to agents.
#[derive(Debug, Clone)]
pub struct IntentRouter {
    routes: Vec<IntentRoute>,
    default_agent: Option<String>,
    min_confidence: f32,
}

impl Default for IntentRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl IntentRouter {
    /// Create a new intent router.
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            default_agent: None,
            min_confidence: 0.5,
        }
    }

    /// Add a route.
    pub fn route(mut self, route: IntentRoute) -> Self {
        self.routes.push(route);
        self
    }

    /// Set the default agent for unmatched intents.
    pub fn default_agent(mut self, agent: impl Into<String>) -> Self {
        self.default_agent = Some(agent.into());
        self
    }

    /// Set minimum confidence threshold.
    pub fn min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Route an intent to an agent.
    pub fn route_intent(&self, intent: &Intent) -> Option<RoutingDecision> {
        let mut matches: Vec<(String, f32, i32)> = Vec::new();
        let input_lower = intent.input.to_lowercase();
        let intent_name_lower = intent.name.to_lowercase();

        for route in &self.routes {
            let mut confidence = 0.0f32;
            let mut matched = false;

            // Check pattern matches
            for pattern in &route.patterns {
                let pattern_lower = pattern.to_lowercase();
                if intent_name_lower.contains(&pattern_lower)
                    || pattern_lower.contains(&intent_name_lower)
                    || intent_name_lower == pattern_lower
                {
                    confidence = confidence.max(0.8);
                    matched = true;
                }
            }

            // Check keyword matches
            for keyword in &route.keywords {
                if input_lower.contains(keyword) {
                    confidence += 0.1;
                    matched = true;
                }
            }

            // Factor in intent confidence
            if matched {
                confidence = (confidence * intent.confidence).clamp(0.0, 1.0);
                if confidence >= self.min_confidence {
                    matches.push((route.agent.clone(), confidence, route.priority));
                }
            }
        }

        if matches.is_empty() {
            // Return default agent if set
            return self
                .default_agent
                .as_ref()
                .map(|agent| RoutingDecision::new(agent.clone(), intent.clone()));
        }

        // Sort by priority (desc), then confidence (desc)
        matches.sort_by(|a, b| {
            b.2.cmp(&a.2)
                .then_with(|| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal))
        });

        let (best_agent, best_confidence, _) = matches.remove(0);

        let mut decision = RoutingDecision::new(best_agent, intent.clone());
        decision.confidence = best_confidence;

        // Add alternatives
        for (agent, confidence, _) in matches.into_iter().take(3) {
            decision = decision.alternative(agent, confidence);
        }

        Some(decision)
    }

    /// Classify input text and route to an agent.
    ///
    /// This is a simple keyword-based classifier. For more sophisticated
    /// intent classification, use an LLM or dedicated NLU service.
    pub fn classify_and_route(&self, input: &str) -> Option<RoutingDecision> {
        let intent = self.classify_intent(input);
        self.route_intent(&intent)
    }

    /// Simple keyword-based intent classification.
    fn classify_intent(&self, input: &str) -> Intent {
        let input_lower = input.to_lowercase();
        let mut best_match: Option<(String, f32)> = None;

        for route in &self.routes {
            let mut score = 0.0f32;
            let mut match_count = 0;

            for pattern in &route.patterns {
                let pattern_lower = pattern.to_lowercase();
                if input_lower.contains(&pattern_lower) {
                    score += 0.5;
                    match_count += 1;
                }
            }

            for keyword in &route.keywords {
                if input_lower.contains(keyword) {
                    score += 0.25;
                    match_count += 1;
                }
            }

            if match_count > 0 {
                score = (score + (match_count as f32 * 0.05)).min(1.0);
                if best_match.is_none() || score > best_match.as_ref().unwrap().1 {
                    let intent_name = route
                        .patterns
                        .first()
                        .cloned()
                        .unwrap_or_else(|| route.agent.clone());
                    best_match = Some((intent_name, score));
                }
            }
        }

        match best_match {
            Some((name, confidence)) => Intent::new(name, input).confidence(confidence),
            None => Intent::new("unknown", input).confidence(0.0),
        }
    }
}

/// Builder for common intent patterns.
pub struct CommonIntents;

impl CommonIntents {
    /// Create a route for greeting intents.
    pub fn greeting(agent: impl Into<String>) -> IntentRoute {
        IntentRoute::new(agent)
            .patterns(["greeting", "hello", "hi", "hey"])
            .keywords([
                "hello",
                "hi",
                "hey",
                "good morning",
                "good afternoon",
                "good evening",
            ])
    }

    /// Create a route for help/question intents.
    pub fn question(agent: impl Into<String>) -> IntentRoute {
        IntentRoute::new(agent)
            .patterns(["question", "help", "ask", "query"])
            .keywords([
                "what", "how", "why", "when", "where", "who", "?", "help", "explain",
            ])
    }

    /// Create a route for code-related intents.
    pub fn code(agent: impl Into<String>) -> IntentRoute {
        IntentRoute::new(agent)
            .patterns(["code", "programming", "development"])
            .keywords([
                "code", "function", "class", "bug", "error", "compile", "debug", "refactor",
            ])
    }

    /// Create a route for search intents.
    pub fn search(agent: impl Into<String>) -> IntentRoute {
        IntentRoute::new(agent)
            .patterns(["search", "find", "lookup"])
            .keywords(["search", "find", "look up", "lookup", "query"])
    }

    /// Create a route for task/action intents.
    pub fn task(agent: impl Into<String>) -> IntentRoute {
        IntentRoute::new(agent)
            .patterns(["task", "action", "do", "execute"])
            .keywords([
                "create", "delete", "update", "add", "remove", "run", "execute",
            ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_route_creation() {
        let route = IntentRoute::new("test_agent")
            .patterns(["greeting", "hello"])
            .keywords(["hi", "hey"])
            .priority(10);

        assert_eq!(route.agent, "test_agent");
        assert_eq!(route.patterns.len(), 2);
        assert_eq!(route.keywords.len(), 2);
        assert_eq!(route.priority, 10);
    }

    #[test]
    fn test_router_route_intent() {
        let router = IntentRouter::new()
            .route(
                IntentRoute::new("greeter")
                    .patterns(["greeting"])
                    .keywords(["hello"]),
            )
            .route(
                IntentRoute::new("helper")
                    .patterns(["question"])
                    .keywords(["help"]),
            )
            .default_agent("fallback");

        let greeting = Intent::new("greeting", "Hello there!");
        let decision = router.route_intent(&greeting);
        assert!(decision.is_some());
        assert_eq!(decision.unwrap().agent, "greeter");
    }

    #[test]
    fn test_router_default_agent() {
        let router = IntentRouter::new()
            .route(IntentRoute::new("greeter").patterns(["greeting"]))
            .default_agent("fallback");

        let unknown = Intent::new("unknown_intent", "xyz123");
        let decision = router.route_intent(&unknown);
        assert!(decision.is_some());
        assert_eq!(decision.unwrap().agent, "fallback");
    }

    #[test]
    fn test_router_classify_and_route() {
        let router = IntentRouter::new()
            .route(CommonIntents::greeting("greeter"))
            .route(CommonIntents::question("helper"))
            .route(CommonIntents::code("coder"))
            .default_agent("general");

        let decision = router.classify_and_route("Hello, how are you?");
        assert!(decision.is_some());
        // Should match greeting due to "hello"
        let d = decision.unwrap();
        assert!(d.agent == "greeter" || d.agent == "helper");
    }

    #[test]
    fn test_router_with_keywords() {
        let router = IntentRouter::new()
            .route(
                IntentRoute::new("weather_agent")
                    .patterns(["weather"])
                    .keywords(["weather", "temperature", "forecast", "rain"]),
            )
            .default_agent("general");

        let decision = router.classify_and_route("What's the weather like today?");
        assert!(decision.is_some());
        assert_eq!(decision.unwrap().agent, "weather_agent");
    }

    #[test]
    fn test_router_priority() {
        let router = IntentRouter::new()
            .route(
                IntentRoute::new("high_priority")
                    .patterns(["test"])
                    .priority(10),
            )
            .route(
                IntentRoute::new("low_priority")
                    .patterns(["test"])
                    .priority(1),
            );

        let intent = Intent::new("test", "test input");
        let decision = router.route_intent(&intent);
        assert!(decision.is_some());
        assert_eq!(decision.unwrap().agent, "high_priority");
    }

    #[test]
    fn test_common_intents() {
        let router = IntentRouter::new()
            .route(CommonIntents::greeting("greeter"))
            .route(CommonIntents::question("helper"))
            .route(CommonIntents::code("coder"))
            .route(CommonIntents::search("searcher"))
            .route(CommonIntents::task("executor"));

        // Test greeting
        let greeting = router.classify_and_route("Hello there!");
        assert!(greeting.is_some());

        // Test code - use input that clearly triggers code keywords without "help"
        let code = router.classify_and_route("I need to debug this function and fix the error");
        assert!(code.is_some());
        assert_eq!(code.unwrap().agent, "coder");
    }
}
