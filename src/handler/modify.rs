use osm_io::osm::model::element::Element;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
use crate::handler::{into_node_element, into_relation_element, into_way_element, Handler};

#[derive(Default)]
pub(crate) struct MetadataRemover;
impl MetadataRemover {
    fn handle_node(&mut self, node: Node) -> Vec<Element> {
        vec![into_node_element(Node::new(node.id(), 0, node.coordinate().clone(), 0, 0, 0, String::default(), node.visible(), node.tags().clone()))]
    }

    fn handle_way(&mut self, way: Way) -> Vec<Element> {
        vec![into_way_element(Way::new(way.id(), 0, 0, 0, 0, String::default(), way.visible(), way.refs().clone(), way.tags().clone()))]
    }

    fn handle_relation(&mut self, relation: Relation) -> Vec<Element> {
        vec![into_relation_element(Relation::new(relation.id(), 0, 0, 0, 0, String::default(), relation.visible(), relation.members().clone(), relation.tags().clone()))]
    }

    fn handle_node2(&mut self, node: &mut Node) {
        *node = Node::new(node.id(), 0, node.coordinate().clone(), 0, 0, 0, String::default(), node.visible(), node.tags().clone())
    }

    fn handle_way2(&mut self, way: &mut Way) {
        *way = Way::new(way.id(), 0, 0, 0, 0, String::default(), way.visible(), way.refs().clone(), way.tags().clone())
    }

    fn handle_relation2(&mut self, relation: &mut Relation) {
        *relation = Relation::new(relation.id(), 0, 0, 0, 0, String::default(), relation.visible(), relation.members().clone(), relation.tags().clone())
    }
}
impl Handler for MetadataRemover {
    fn name(&self) -> String {
        "MetadataRemover".to_string()
    }
    fn handle_element(&mut self, element: Element) -> Vec<Element> {
        match element {
            Element::Node { node } => { self.handle_node(node) }
            Element::Way { way } => { self.handle_way(way) }
            Element::Relation { relation } => { self.handle_relation(relation) }
            Element::Sentinel => vec![]
        }
    }

    fn handle_nodes(&mut self, mut elements: Vec<Node>) -> Vec<Node> {
        //for element in elements {
        //    *element = self.handle_node2(element);
        //}
        elements.iter_mut().for_each(|element| self.handle_node2(element));
        elements
    }

    fn handle_ways(&mut self, mut elements: Vec<Way>) -> Vec<Way> {
        elements.iter_mut().for_each(|element| self.handle_way2(element));
        elements
    }

    fn handle_relations(&mut self, mut elements: Vec<Relation>) -> Vec<Relation> {
        elements.iter_mut().for_each(|element| self.handle_relation2(element));
        elements
    }
}


#[cfg(test)]
mod test {
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::element::Element;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::relation::{Member, MemberData, Relation};
    use osm_io::osm::model::tag::Tag;
    use osm_io::osm::model::way::Way;
    use crate::handler::modify::MetadataRemover;

    #[test]
    fn metadata_remover_node() {
        let mut metadata_remover = MetadataRemover;
        let binding = metadata_remover.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,vec![
            Tag::new("a".to_string(), "x".to_string()),
            Tag::new("b".to_string(), "y".to_string()),
        ]));
        match binding.first().unwrap() {
            Element::Node { node } => {
                assert_eq!(node.id(), 1);
                assert_eq!(node.version(), 0);
                assert_eq!(node.coordinate().lat(), 1.0f64);
                assert_eq!(node.coordinate().lon(), 1.1f64);
                assert_eq!(node.timestamp(), 0);
                assert_eq!(node.changeset(), 0);
                assert_eq!(node.uid(), 0);
                assert_eq!(node.user(), &String::default());
                assert!(node.visible());
                assert_eq!(node.tags()[0].k(), &"a".to_string());
                assert_eq!(node.tags()[0].v(), &"x".to_string());
                assert_eq!(node.tags()[1].k(), &"b".to_string());
                assert_eq!(node.tags()[1].v(), &"y".to_string());
            }
            _ => panic!("expected onde node")
        }
    }

    #[test]
    fn metadata_remover_way() {
        let mut metadata_remover = MetadataRemover;
        let binding = metadata_remover.handle_way(Way::new(1, 1, 1, 1, 1, "user".to_string(), true, vec![4, 6], vec![
            Tag::new("a".to_string(), "x".to_string()),
            Tag::new("b".to_string(), "y".to_string()),
        ]));
        match binding.first().unwrap() {
            Element::Way {way} => {
                assert_eq!(way.id(), 1);
                assert_eq!(way.version(), 0);
                assert_eq!(way.timestamp(), 0);
                assert_eq!(way.changeset(), 0);
                assert_eq!(way.uid(), 0);
                assert_eq!(way.user(), &String::default());
                assert!(way.visible());
                assert_eq!(way.refs()[0], 4);
                assert_eq!(way.refs()[1], 6);
                assert_eq!(way.tags()[0].k(), &"a".to_string());
                assert_eq!(way.tags()[0].v(), &"x".to_string());
                assert_eq!(way.tags()[1].k(), &"b".to_string());
                assert_eq!(way.tags()[1].v(), &"y".to_string());
            }
            _ => {panic!("expected a way")}
        }
    }

    #[test]
    fn metadata_remover_relation() {
        let mut metadata_remover = MetadataRemover;
        let binding = metadata_remover.handle_relation(Relation::new(1, 1, 1, 1, 1, "user".to_string(), true, vec![
            Member::Node { member: MemberData::new(5, "a".to_string()) },
            Member::Node { member: MemberData::new(6, "b".to_string()) },
            Member::Way { member: MemberData::new(10, "b".to_string()) },
            Member::Relation { member: MemberData::new(20, "b".to_string()) },
        ], vec![
            Tag::new("a".to_string(), "x".to_string()),
            Tag::new("b".to_string(), "y".to_string()),
        ]));
        match binding.first().unwrap() {
            Element::Relation { relation } => {
                assert_eq!(relation.id(), 1);
                assert_eq!(relation.version(), 0);
                assert_eq!(relation.timestamp(), 0);
                assert_eq!(relation.changeset(), 0);
                assert_eq!(relation.uid(), 0);
                assert_eq!(relation.user(), &String::default());
                assert!(relation.visible());
                assert_eq!(relation.members()[0], Member::Node { member: MemberData::new(5, "a".to_string()) });
                assert_eq!(relation.members()[1], Member::Node { member: MemberData::new(6, "b".to_string()) });
                assert_eq!(relation.members()[2], Member::Way { member: MemberData::new(10, "b".to_string()) });
                assert_eq!(relation.members()[3], Member::Relation { member: MemberData::new(20, "b".to_string()) });
                assert_eq!(relation.tags()[0].k(), &"a".to_string());
                assert_eq!(relation.tags()[0].v(), &"x".to_string());
                assert_eq!(relation.tags()[1].k(), &"b".to_string());
                assert_eq!(relation.tags()[1].v(), &"y".to_string());
            }
            _ => panic!("expected a realtion!")
        }
    }
}