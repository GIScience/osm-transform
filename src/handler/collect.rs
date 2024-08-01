use bit_vec::BitVec;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::relation::{Member, Relation};
use osm_io::osm::model::way::Way;

use crate::handler::{HandlerResult, HIGHEST_NODE_ID, into_node_element, into_relation_element, into_way_element, Handler};

pub(crate) struct ReferencedNodeIdCollector {
    referenced_node_ids: BitVec
}
impl ReferencedNodeIdCollector {
    pub(crate) fn default() -> Self {
        Self::with_capacity(HIGHEST_NODE_ID as usize)
    }
    fn with_capacity(nbits: usize) -> Self {
        ReferencedNodeIdCollector {
            referenced_node_ids: BitVec::from_elem(nbits, false)
        }
    }
    fn handle_way(&mut self, way: Way) -> Vec<Element> {
        log::trace!("xxxxxxxxxxxxxxxxx way");
        for id in way.refs() {
            let idc = id.clone();
            self.referenced_node_ids.set(idc as usize, true);
        }
        vec![into_way_element(way)]
    }
    fn handle_relation(&mut self, relation: Relation) -> Vec<Element> {
        log::trace!("xxxxxxxxxxxxxxxxx relation");
        for member in relation.members() {
            match member {
                Member::Node { member } => {
                    log::trace!("relation {} references node {} - set true in bitmap", &relation.id(), &member.id());
                    self.referenced_node_ids.set(member.id().clone() as usize, true);
                }
                Member::Way { .. } => {}
                Member::Relation { .. } => {}
            }

        }
        vec![into_relation_element(relation)]
    }
}
impl Handler for ReferencedNodeIdCollector {
    fn name(&self) -> String {
        "ReferencedNodeIdCollector".to_string()
    }
    fn handle_element(&mut self, element: Element) -> Vec<Element> {
        match element {
            Element::Node { node } => vec![into_node_element(node)],
            Element::Way { way } => self.handle_way(way),
            Element::Relation { relation } => self.handle_relation(relation),
            _ => vec![]
        }
    }
    fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        log::debug!("cloning node_ids of ReferencedNodeIdCollector with len={} into HandlerResult ", self.referenced_node_ids.len());
        result.node_ids = self.referenced_node_ids.clone();//todo check if clone is necessary
        result
    }
}

#[cfg(test)]
mod test {
    use crate::handler::{HIGHEST_NODE_ID, Handler};
    use crate::handler::tests::{simple_node_element, TestOnlyIdCollector};

    #[test]
    fn node_id_collector(){
        let mut collector = TestOnlyIdCollector::new(10);
        assert_eq!(10, collector.node_ids.len());
        collector.handle_element(simple_node_element(2, vec![]));
        assert_eq!(false, collector.node_ids.get(0).unwrap_or(false));
        assert_eq!(false, collector.node_ids.get(1).unwrap_or(false));
        assert_eq!(true, collector.node_ids.get(2).unwrap_or(false));
    }
    #[test]
    #[should_panic(expected = "index out of bounds: 12 >= 10")]
    fn node_id_collector_out_of_bounds(){
        let mut collector = TestOnlyIdCollector::new(10);
        collector.handle_element(simple_node_element(12,vec![]));
    }
    #[test]
    fn node_id_collector_out_of_bounds_real(){
        let mut collector = TestOnlyIdCollector::new(HIGHEST_NODE_ID as usize);

        collector.handle_element(simple_node_element(1, vec![]));
        assert_eq!(false, collector.node_ids.get(0).unwrap_or(false));
        assert_eq!(true, collector.node_ids.get(1).unwrap_or(false));
        assert_eq!(false, collector.node_ids.get(2).unwrap_or(false));
        assert_eq!(false, collector.node_ids.get(11414456780).unwrap_or(false));

        collector.handle_element(simple_node_element(11414456780, vec![]));
        assert_eq!(true, collector.node_ids.get(11414456780).unwrap_or(false));
    }

}