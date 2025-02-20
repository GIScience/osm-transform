use std::collections::HashMap;
use bit_vec::BitVec;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::tag::Tag;
use osm_io::osm::model::way::Way;
use regex::Regex;
use crate::handler::{OsmElementTypeSelection, Handler, HandlerResult};
use crate::handler::predicate::{HasOneOfTagKeysPredicate, HasTagKeyValuePredicate, HasNoneOfTagKeysPredicate};

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum FilterType {
    AcceptMatching,
    RemoveMatching,
}

pub(crate) struct TagValueBasedOsmElementsFilter {
    pub handle_types: OsmElementTypeSelection,
    pub tag_key: String,
    pub tag_value_regex: Regex,
    pub filter_type: FilterType,
}
impl TagValueBasedOsmElementsFilter {
    #[allow(dead_code)]
    pub(crate) fn new(handle_types: OsmElementTypeSelection, tag_key: String, tag_value_regex: Regex, filter_type: FilterType) -> Self {
        Self {
            handle_types,
            tag_key,
            tag_value_regex,
            filter_type,
        }
    }
    fn accept_by_tags(&mut self, tags: &Vec<Tag>) -> bool {
        let mut matched = false;
        for tag in tags {
            if self.tag_key.eq(tag.k()) && self.tag_value_regex.is_match(tag.v()) {
                matched = true;
                break
            }
        }
        match self.filter_type {
            FilterType::AcceptMatching =>  { matched }
            FilterType::RemoveMatching =>  {!matched }
        }
    }
}
impl Handler for TagValueBasedOsmElementsFilter {
    fn name(&self) -> String { "TagValueBasedOsmElementsFilter".to_string() }

    fn handle_result(&mut self, result: &mut HandlerResult) {
        if self.handle_types.node { result.nodes.retain(|node| self.accept_by_tags(node.tags())) };
        if self.handle_types.way { result.ways.retain(|way| self.accept_by_tags(way.tags())) };
        if self.handle_types.relation { result.relations.retain(|relation| self.accept_by_tags(relation.tags())) };
    }
}



pub(crate) struct TagKeyBasedOsmElementsFilter {
    pub handle_types: OsmElementTypeSelection,
    pub tag_keys: Vec<String>,
    pub filter_type: FilterType,
}
impl TagKeyBasedOsmElementsFilter {
    #[allow(dead_code)]

    pub(crate) fn new(handle_types: OsmElementTypeSelection, tag_keys: Vec<String>, filter_type: FilterType) -> Self {
        Self {
            handle_types,
            tag_keys,
            filter_type,
        }
    }
    fn accept_by_tags(&mut self, tags: &Vec<Tag>) -> bool {
        let contains_any_key = tags.iter().any(|tag| self.tag_keys.contains(tag.k()));
        match self.filter_type {
            FilterType::AcceptMatching => {
                contains_any_key
            }
            FilterType::RemoveMatching => {
                !contains_any_key
            }
        }
    }
}
impl Handler for TagKeyBasedOsmElementsFilter {
    fn name(&self) -> String { "TagKeyBasedOsmElementsFilter".to_string() }

    fn handle_result(&mut self, result: &mut HandlerResult) {
        result.nodes.retain(|node| self.accept_by_tags(node.tags()));
        result.ways.retain(|way| self.accept_by_tags(way.tags()));
        result.relations.retain(|relation| self.accept_by_tags(relation.tags()));
    }
}


pub(crate) struct TagFilterByKey {
    pub handle_types: OsmElementTypeSelection,
    pub key_regex: Regex,
    pub filter_type: FilterType,
}
impl TagFilterByKey {
    pub(crate) fn new(handle_types: OsmElementTypeSelection, key_regex: Regex, filter_type: FilterType) -> Self {
        Self {
            handle_types,
            key_regex,
            filter_type,
        }
    }
    fn filter_tags(&mut self, tags: &mut Vec<Tag>) {
        match self.filter_type {
            FilterType::AcceptMatching => {
                tags.retain(|tag| self.key_regex.is_match(&tag.k()));
            }
            FilterType::RemoveMatching => {
                tags.retain(|tag| !self.key_regex.is_match(&tag.k()));
            }
        }
    }
}
impl Handler for TagFilterByKey {
    fn name(&self) -> String {
        "TagFilterByKey".to_string()
    }

    fn handle_nodes(&mut self, mut nodes: Vec<Node>) -> Vec<Node> {
        if self.handle_types.node {
            for node in nodes.iter_mut() {
                self.filter_tags(node.tags_mut());
            }
        }
        nodes
    }

    fn handle_ways(&mut self, mut ways: Vec<Way>) -> Vec<Way> {
        if self.handle_types.way {
            for way in ways.iter_mut() {
                self.filter_tags( way.tags_mut());
            }
        }
        ways
    }

    fn handle_relations(&mut self, mut relations: Vec<Relation>) -> Vec<Relation> {
        if self.handle_types.relation {
            for relation in relations.iter_mut() {
                self.filter_tags(&mut relation.tags_mut());
            }
        }
        relations
    }
}



pub(crate) struct AllElementsFilter {
    pub handle_types: OsmElementTypeSelection,
}
impl Handler for AllElementsFilter {
    fn name(&self) -> String {
        "AllElementsFilter".to_string()
    }

    fn handle_result(&mut self, result: &mut HandlerResult) {
        if self.handle_types.node { result.nodes.clear() };
        if self.handle_types.way { result.ways.clear() };
        if self.handle_types.relation { result.relations.clear() };
    }
}


pub(crate) struct NodeIdFilter {}
impl NodeIdFilter {}

impl Handler for NodeIdFilter {
    fn name(&self) -> String {
        "NodeIdFilter".to_string()
    }
    fn handle_result(&mut self, result: &mut HandlerResult) {
        result.nodes.retain(|node| result.node_ids.get(node.id() as usize) == Some(true));
    }
}




pub(crate) struct ComplexElementsFilter { //TODO rename to WayRelationFilterForRouting
    pub has_good_key_predicate: HasOneOfTagKeysPredicate,     //TODO add cli option (parse comma separated list)
    pub has_good_key_value_predicate: HasTagKeyValuePredicate,//TODO add cli option (parse comma:colon separated list)
    pub has_bad_key_predicate: HasNoneOfTagKeysPredicate,     //TODO add cli option (parse comma separated list)
}
impl ComplexElementsFilter {
    pub(crate) fn new(
        has_good_key_predicate: HasOneOfTagKeysPredicate,
        has_good_key_value_predicate: HasTagKeyValuePredicate,
        has_bad_key_predicate: HasNoneOfTagKeysPredicate) -> Self {
        Self {
            has_good_key_predicate,
            has_good_key_value_predicate,
            has_bad_key_predicate,
        }
    }

    pub(crate) fn ors_default() -> Self{
        let mut key_values = HashMap::new();
        key_values.insert("railway".to_string(), "platform".to_string());
        key_values.insert("public_transport".to_string(), "platform".to_string());
        key_values.insert("man_made".to_string(), "pier".to_string());

        ComplexElementsFilter::new(
            HasOneOfTagKeysPredicate { keys: vec!["highway".to_string(), "route".to_string()] },
            HasTagKeyValuePredicate { key_values: key_values },
            HasNoneOfTagKeysPredicate {
                keys: vec![
                    "building".to_string(),
                    "landuse".to_string(),
                    "boundary".to_string(),
                    "natural".to_string(),
                    "place".to_string(),
                    "waterway".to_string(),
                    "aeroway".to_string(),
                    "aviation".to_string(),
                    "military".to_string(),
                    "power".to_string(),
                    "communication".to_string(),
                    "man_made".to_string()]
            })
    }
    fn accept_by_tags(&mut self, tags: &Vec<Tag>) -> bool {
        self.has_good_key_predicate.test(tags) ||
            self.has_good_key_value_predicate.test(tags) ||
            self.has_bad_key_predicate.test(tags)
    }

    fn is_way_accepted(&mut self, way: &Way) -> bool {
        match self.accept_by_tags(way.tags()) {
            true => {
                log::trace!("accepting way {}", way.id());
                true
            }
            false => {
                log::trace!("removing way {}", way.id());
                false
            }
        }
    }

    fn is_relation_accepted(&mut self, relation: &Relation) -> bool {
        match self.accept_by_tags(relation.tags()) {
            true => {
                log::trace!("accepting relation {}", relation.id());
                true
            }
            false => {
                log::trace!("removing relation {}", relation.id());
                false
            }
        }
    }
}
impl Handler for ComplexElementsFilter {
    fn name(&self) -> String {
        "ComplexElementsFilter".to_string()
    }

    fn handle_ways(&mut self, mut elements: Vec<Way>) -> Vec<Way> {
        elements.retain(|way| self.is_way_accepted(way));
        elements
    }

    fn handle_relations(&mut self, mut elements: Vec<Relation>) -> Vec<Relation> {
        elements.retain(|relation| self.is_relation_accepted(relation));
        elements
    }
}

#[cfg(test)]
mod test {
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::tag::Tag;
    use osm_io::osm::model::way::Way;
    use regex::Regex;
    use crate::handler::filter::{ComplexElementsFilter, FilterType, TagFilterByKey, TagKeyBasedOsmElementsFilter};
    use crate::handler::{Handler, HandlerResult, OsmElementTypeSelection};

    #[test]
    fn test_tag_filter_by_key_with_remove_matching() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::node_only(),
            Regex::new(".*bad.*").unwrap(),
            FilterType::RemoveMatching);

        let handled_nodes = tag_filter.handle_nodes(vec![Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                             vec![
                                                                 Tag::new("bad".to_string(), "hotzenplotz".to_string()),
                                                                 Tag::new("good".to_string(), "kasper".to_string()),
                                                                 Tag::new("more-bad".to_string(), "vader".to_string()),
                                                                 Tag::new("more-good".to_string(), "grandma".to_string()),
                                                                 Tag::new("badest".to_string(), "voldemort".to_string()),
                                                             ])]);
        assert_eq!(1, handled_nodes.len());
        let node = handled_nodes.get(0).unwrap();
        assert_eq!(node.tags().len(), 2);
        assert_eq!(node.tags()[0].k(), &"good");
        assert_eq!(node.tags()[0].v(), &"kasper");
        assert_eq!(node.tags()[1].k(), &"more-good");
        assert_eq!(node.tags()[1].v(), &"grandma");
    }

    #[test]
    fn test_tag_filter_by_key_with_remove_matching_complex_regex() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::node_only(),
            Regex::new("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia").unwrap(),
            FilterType::RemoveMatching);

        let handled_nodes = tag_filter.handle_nodes(vec![Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                             vec![
                                                                 Tag::new("closed:source".to_string(), "bad".to_string()),
                                                                 Tag::new("source".to_string(), "bad".to_string()),
                                                                 Tag::new("source:x".to_string(), "bad".to_string()),
                                                                 Tag::new("x:source:y".to_string(), "bad".to_string()),
                                                                 Tag::new("opensource".to_string(), "bad".to_string()), //really?
                                                                 Tag::new("note".to_string(), "bad".to_string()),
                                                                 Tag::new("url".to_string(), "bad".to_string()),
                                                                 Tag::new("created_by".to_string(), "bad".to_string()),
                                                                 Tag::new("fixme".to_string(), "bad".to_string()),
                                                                 Tag::new("wikipedia".to_string(), "bad".to_string()),
                                                                 Tag::new("wikimedia".to_string(), "good".to_string()),
                                                             ])]);
        assert_eq!(1, handled_nodes.len());
        let node = handled_nodes.get(0).unwrap();
        assert_eq!(node.tags().len(), 1);
        for tag in node.tags() {
            assert_eq!(tag.v(), "good")
        }
    }

    #[test]
    fn test_tag_filter_by_key_with_keep_matching() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::all(),
            Regex::new(".*good.*").unwrap(),
            FilterType::AcceptMatching);

        let handled_nodes = tag_filter.handle_nodes(vec![Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                             vec![
                                                                 Tag::new("bad".to_string(), "hotzenplotz".to_string()),
                                                                 Tag::new("good".to_string(), "kasper".to_string()),
                                                                 Tag::new("more-bad".to_string(), "vader".to_string()),
                                                                 Tag::new("more-good".to_string(), "grandma".to_string()),
                                                                 Tag::new("badest".to_string(), "voldemort".to_string()),
                                                             ])]);
        assert_eq!(1, handled_nodes.len());
        // let node = &handled_nodes[0];
        let node = handled_nodes.get(0).unwrap();
        assert_eq!(node.tags().len(), 2);
        assert_eq!(node.tags().len(), 2);
        assert_eq!(node.tags()[0].k(), &"good");
        assert_eq!(node.tags()[0].v(), &"kasper");
        assert_eq!(node.tags()[1].k(), &"more-good");
        assert_eq!(node.tags()[1].v(), &"grandma");
    }
    #[test]
    fn test_tag_filter_by_key_with_node_not_handled() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::way_only(),
            Regex::new(".*").unwrap(),
            FilterType::RemoveMatching);

        let handled_nodes = tag_filter.handle_nodes(vec![Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                             vec![
                                                                 Tag::new("a".to_string(), "1".to_string()),
                                                                 Tag::new("b".to_string(), "2".to_string()),
                                                                 Tag::new("c".to_string(), "3".to_string()),
                                                             ])]);
        assert_eq!(1, handled_nodes.len());
        let node = handled_nodes.get(0).unwrap();
        assert_eq!(node.tags().len(), 3);
    }
    #[test]
    fn test_tag_filter_by_key_with_node_handled() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::all(),
            Regex::new(".*").unwrap(),
            FilterType::RemoveMatching);

        let handled_nodes = tag_filter.handle_nodes(vec![Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                             vec![
                                                                 Tag::new("a".to_string(), "1".to_string()),
                                                                 Tag::new("b".to_string(), "2".to_string()),
                                                                 Tag::new("c".to_string(), "3".to_string()),
                                                             ])]);
        assert_eq!(handled_nodes.len(), 1);
        let node = handled_nodes.get(0).unwrap();
        assert_eq!(node.tags().len(), 0);
    }
    #[test]
    fn filter_elements_remove_by_keys() {
        let mut filter = TagKeyBasedOsmElementsFilter::new(
            OsmElementTypeSelection::all(),
            vec!["bad".to_string(), "ugly".to_string()],
            FilterType::RemoveMatching);
        let mut result = HandlerResult::default();

        result.nodes.push(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                    vec![
                                        Tag::new("good".to_string(), "1".to_string()),
                                        Tag::new("bad".to_string(), "2".to_string()),
                                    ]));
        filter.handle_result(&mut result);
        assert_eq!(result.nodes.len(), 0);

        result.clear_elements();
        result.nodes.push(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                    vec![
                                        Tag::new("good".to_string(), "1".to_string()),
                                        Tag::new("nice".to_string(), "2".to_string()),
                                    ]));
        filter.handle_result(&mut result);
        assert_eq!(result.nodes.len(), 1);

        result.clear_elements();
        result.nodes.push(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                    vec![
                                        Tag::new("ugly".to_string(), "1".to_string()),
                                        Tag::new("bad".to_string(), "2".to_string()),
                                    ]));
        filter.handle_result(&mut result);
        assert_eq!(result.nodes.len(), 0);

    }

    #[test]
    fn filter_elements_accept_by_keys() {
        let mut filter = TagKeyBasedOsmElementsFilter::new(
            OsmElementTypeSelection::all(),
            vec!["bad".to_string(), "ugly".to_string()],
            FilterType::AcceptMatching,
        );
        let mut result = HandlerResult::default();

        result.nodes.push(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                    vec![
                                        Tag::new("good".to_string(), "1".to_string()),
                                        Tag::new("bad".to_string(), "2".to_string()),
                                    ]));
        filter.handle_result(&mut result);
        assert_eq!(result.nodes.len(), 1);

        result.clear_elements();
        result.nodes.push(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                    vec![
                                        Tag::new("good".to_string(), "1".to_string()),
                                        Tag::new("nice".to_string(), "2".to_string()),
                                    ]));
        filter.handle_result(&mut result);
        assert_eq!(result.nodes.len(), 0);

        result.clear_elements();
        result.nodes.push(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                    vec![
                                        Tag::new("ugly".to_string(), "1".to_string()),
                                        Tag::new("bad".to_string(), "2".to_string()),
                                    ]));
        filter.handle_result(&mut result);
        assert_eq!(result.nodes.len(), 1);
    }

    #[test]
    fn complex_filter_with_ors_default() {
        let mut filter = ComplexElementsFilter::ors_default();
        // has key to keep and key-value to keep, bad key 'building' should not take effect => should be accepted
        assert_eq!(filter.handle_ways(vec![Way::new(1, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                              vec![
                                                  Tag::new("route".to_string(), "xyz".to_string()),
                                                  Tag::new("railway".to_string(), "platform".to_string()),
                                                  Tag::new("building".to_string(), "x".to_string()),
                                              ])]).len(), 1);

        // has key to keep, bad key 'building' should not take effect => should be accepted
        assert_eq!(filter.handle_ways(vec![Way::new(2, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                              vec![
                                                  Tag::new("route".to_string(), "xyz".to_string()),
                                                  Tag::new("building".to_string(), "x".to_string()),
                                              ])]).len(), 1);

        // has key-value to keep, bad key 'building' should not take effect => should be accepted
        assert_eq!(filter.handle_ways(vec![Way::new(3, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                              vec![
                                                  Tag::new("railway".to_string(), "platform".to_string()),
                                                  Tag::new("building".to_string(), "x".to_string()),
                                              ])]).len(), 1);

        // has no key or key-value to keep, but also no bad key => should be accepted
        assert_eq!(filter.handle_ways(vec![Way::new(4, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                              vec![
                                                  Tag::new("railway".to_string(), "wrong-value".to_string()),
                                                  Tag::new("something".to_string(), "else".to_string()),
                                              ])]).len(), 1);

        // has no key or key-value to keep, some other key, but also one bad key => should be filtered
        assert_eq!(filter.handle_ways(vec![Way::new(5, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                              vec![
                                                  Tag::new("railway".to_string(), "wrong-value".to_string()),
                                                  Tag::new("something".to_string(), "else".to_string()),
                                                  Tag::new("building".to_string(), "x".to_string()),
                                              ])]).len(), 0);

        // has only one bad key => should be filtered
        assert_eq!(filter.handle_ways(vec![Way::new(6, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                              vec![
                                                  Tag::new("building".to_string(), "x".to_string()),
                                              ])]).len(), 0);

        // has only one other key => should be accepted
        assert_eq!(filter.handle_ways(vec![Way::new(7, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                              vec![
                                                  Tag::new("something".to_string(), "x".to_string()),
                                              ])]).len(), 1);
    }

}
