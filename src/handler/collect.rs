use bit_vec::BitVec;
use osm_io::osm::model::relation::{Member, Relation};
use osm_io::osm::model::way::Way;

use crate::handler::{HandlerResult, HIGHEST_NODE_ID, Handler};

pub(crate) struct ReferencedNodeIdCollector {
    count_unique: usize,
}
impl ReferencedNodeIdCollector {
    pub(crate) fn default() -> Self {
        Self {
            count_unique: 0,
        }
    }


    fn handle_ways_result(&mut self, result: &mut HandlerResult)  {
        for element in & result.ways {
            for &id in element.refs() {
                self.add_node_id(&mut result.node_ids, id);
            }
        }
    }

    fn handle_relations_result(&mut self, result: &mut HandlerResult)  {
        for element in &mut result.relations{
            for member in element.members() {
                match member {
                    Member::Node { member } => {
                        self.add_node_id(&mut result.node_ids, member.id());
                    }
                    Member::Way { .. } => {}
                    Member::Relation { .. } => {}
                }
            }
        }
    }
    fn add_node_id(&mut self, node_ids: &mut BitVec, id: i64)  {
        if matches!(node_ids.get(id as usize), Some(false)) { self.count_unique += 1 }
        node_ids.set(id as usize, true);
    }
}
impl Handler for ReferencedNodeIdCollector {
    fn name(&self) -> String {
        "ReferencedNodeIdCollector".to_string()
    }

    fn handle_result(&mut self, result: &mut HandlerResult) {
        self.handle_ways_result(result);
        self.handle_relations_result(result);
    }
}

#[cfg(test)]
mod test {
    use crate::handler::{HIGHEST_NODE_ID, Handler};
    use crate::handler::tests::{simple_node, TestOnlyIdCollector};

    #[test]
    fn node_id_collector(){
        let mut collector = TestOnlyIdCollector::new(10);
        assert_eq!(10, collector.node_ids.len());
        collector.handle_nodes(vec![simple_node(2, vec![])]);
        assert_eq!(false, collector.node_ids.get(0).unwrap_or(false));
        assert_eq!(false, collector.node_ids.get(1).unwrap_or(false));
        assert_eq!(true, collector.node_ids.get(2).unwrap_or(false));
    }
    #[test]
    #[should_panic(expected = "index out of bounds: 12 >= 10")]
    fn node_id_collector_out_of_bounds(){
        let mut collector = TestOnlyIdCollector::new(10);
        collector.handle_nodes(vec![simple_node(12,vec![])]);
    }
    #[test]
    fn node_id_collector_out_of_bounds_real(){
        let mut collector = TestOnlyIdCollector::new(HIGHEST_NODE_ID as usize);

        collector.handle_nodes(vec![simple_node(1, vec![])]);
        assert_eq!(false, collector.node_ids.get(0).unwrap_or(false));
        assert_eq!(true, collector.node_ids.get(1).unwrap_or(false));
        assert_eq!(false, collector.node_ids.get(2).unwrap_or(false));
        assert_eq!(false, collector.node_ids.get(11414456780).unwrap_or(false));

        collector.handle_nodes(vec![simple_node(11414456780, vec![])]);
        assert_eq!(true, collector.node_ids.get(11414456780).unwrap_or(false));
    }

}