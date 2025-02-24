use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
use crate::handler::{Handler, HandlerResult};

#[derive(Default)]
pub(crate) struct MetadataRemover;
impl MetadataRemover {
    fn handle_node(&mut self, node: &mut Node) {
        *node = Node::new(node.id(), 0, node.coordinate().clone(), 0, 0, 0, String::default(), node.visible(), node.tags().clone())
    }

    fn handle_way(&mut self, way: &mut Way) {
        *way = Way::new(way.id(), 0, 0, 0, 0, String::default(), way.visible(), way.refs().clone(), way.tags().clone())
    }

    fn handle_relation(&mut self, relation: &mut Relation) {
        *relation = Relation::new(relation.id(), 0, 0, 0, 0, String::default(), relation.visible(), relation.members().clone(), relation.tags().clone())
    }
}
impl Handler for MetadataRemover {
    fn name(&self) -> String {
        "MetadataRemover".to_string()
    }

    fn handle(&mut self, result: &mut HandlerResult) {
        result.nodes.iter_mut().for_each(|element| self.handle_node(element));
        result.ways.iter_mut().for_each(|element| self.handle_way(element));
        result.relations.iter_mut().for_each(|element| self.handle_relation(element));
    }
}


#[cfg(test)]
mod test {
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::relation::{Member, MemberData, Relation};
    use osm_io::osm::model::tag::Tag;
    use osm_io::osm::model::way::Way;
    use crate::handler::{Handler, HandlerResult};
    use crate::handler::modify::MetadataRemover;

    #[test]
    fn metadata_remover_node() {
        let mut metadata_remover = MetadataRemover::default();
        let mut result = HandlerResult::default();
        result.nodes.push(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,vec![
            Tag::new("a".to_string(), "x".to_string()),
            Tag::new("b".to_string(), "y".to_string()),
        ]));
        metadata_remover.handle(&mut result);
        let node = &result.nodes[0];

        assert_eq!(node.id(), 1);
        assert_eq!(node.version(), 0);
        assert_eq!(node.coordinate().lat(), 1.0f64);
        assert_eq!(node.coordinate().lon(), 1.1f64);
        assert_eq!(node.timestamp(), 0);
        assert_eq!(node.changeset(), 0);
        assert_eq!(node.uid(), 0);
        assert_eq!(node.user(), &String::default());
        assert_eq!(node.visible(), true);
        assert_eq!(node.tags()[0].k(), &"a".to_string());
        assert_eq!(node.tags()[0].v(), &"x".to_string());
        assert_eq!(node.tags()[1].k(), &"b".to_string());
        assert_eq!(node.tags()[1].v(), &"y".to_string());
    }

    #[test]
    fn metadata_remover_way() {
        let mut metadata_remover = MetadataRemover::default();
        let mut result = HandlerResult::default();
        result.ways.push(Way::new(1, 1, 1, 1, 1, "user".to_string(), true, vec![4, 6], vec![
            Tag::new("a".to_string(), "x".to_string()),
            Tag::new("b".to_string(), "y".to_string()),
        ]));
        metadata_remover.handle(&mut result);
        let way = &result.ways[0];
        assert_eq!(way.id(), 1);
        assert_eq!(way.version(), 0);
        assert_eq!(way.timestamp(), 0);
        assert_eq!(way.changeset(), 0);
        assert_eq!(way.uid(), 0);
        assert_eq!(way.user(), &String::default());
        assert_eq!(way.visible(), true);
        assert_eq!(way.refs()[0], 4);
        assert_eq!(way.refs()[1], 6);
        assert_eq!(way.tags()[0].k(), &"a".to_string());
        assert_eq!(way.tags()[0].v(), &"x".to_string());
        assert_eq!(way.tags()[1].k(), &"b".to_string());
        assert_eq!(way.tags()[1].v(), &"y".to_string());
    }

    #[test]
    fn metadata_remover_relation() {
        let mut metadata_remover = MetadataRemover::default();
        let mut result = HandlerResult::default();
        result.relations.push(Relation::new(1, 1, 1, 1, 1, "user".to_string(), true, vec![
            Member::Node { member: MemberData::new(5, "a".to_string()) },
            Member::Node { member: MemberData::new(6, "b".to_string()) },
            Member::Way { member: MemberData::new(10, "b".to_string()) },
            Member::Relation { member: MemberData::new(20, "b".to_string()) },
        ], vec![
            Tag::new("a".to_string(), "x".to_string()),
            Tag::new("b".to_string(), "y".to_string()),
        ]));
        metadata_remover.handle(&mut result);
        let relation = &result.relations[0];
        assert_eq!(relation.id(), 1);
        assert_eq!(relation.version(), 0);
        assert_eq!(relation.timestamp(), 0);
        assert_eq!(relation.changeset(), 0);
        assert_eq!(relation.uid(), 0);
        assert_eq!(relation.user(), &String::default());
        assert_eq!(relation.visible(), true);
        assert_eq!(relation.members()[0], Member::Node { member: MemberData::new(5, "a".to_string()) });
        assert_eq!(relation.members()[1], Member::Node { member: MemberData::new(6, "b".to_string()) });
        assert_eq!(relation.members()[2], Member::Way { member: MemberData::new(10, "b".to_string()) });
        assert_eq!(relation.members()[3], Member::Relation { member: MemberData::new(20, "b".to_string()) });
        assert_eq!(relation.tags()[0].k(), &"a".to_string());
        assert_eq!(relation.tags()[0].v(), &"x".to_string());
        assert_eq!(relation.tags()[1].k(), &"b".to_string());
        assert_eq!(relation.tags()[1].v(), &"y".to_string());
    }
}