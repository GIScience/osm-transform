use osm_io::osm::model::node::Node;
use osm_io::osm::model::coordinate::Coordinate;
use osm_io::osm::model::tag::Tag;

#[derive(Debug, Clone)]
pub struct MutableNode {
    pub(crate) id: i64,
    pub(crate) version: i32,
    pub(crate) coordinate: Coordinate,
    pub(crate) timestamp: i64,
    pub(crate) changeset: i64,
    pub(crate) uid: i32,
    pub(crate) user: String,
    pub(crate) visible: bool,
    pub(crate) tags: Vec<Tag>,
}

impl MutableNode {
    pub fn new(node: &mut Node) -> MutableNode {
        MutableNode {
            id: node.id(),
            version: node.version(),
            coordinate: node.coordinate().clone(),
            timestamp: node.timestamp(),
            changeset: node.changeset(),
            uid: node.uid(),
            user: node.user().to_string(),
            visible: node.visible(),
            tags: node.take_tags(),
        }
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn version(&self) -> i32 {
        self.version
    }

    pub fn coordinate(&self) -> &Coordinate {
        &self.coordinate
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn changeset(&self) -> i64 {
        self.changeset
    }

    pub fn uid(&self) -> i32 {
        self.uid
    }

    pub fn user(&self) -> &String {
        &self.user
    }

    pub fn take_user(&mut self) -> String {
        std::mem::take(&mut self.user)
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn tags(&self) -> &Vec<Tag> {
        &self.tags
    }

    pub fn tags_mut(&mut self) -> &mut Vec<Tag> {
        &mut self.tags
    }

    pub fn take_tags(&mut self) -> Vec<Tag> {
        std::mem::take(&mut self.tags)
    }

    pub fn make_node(&mut self) -> Node {
        Node::new(self.id, self.version, self.coordinate.clone(), self.timestamp, self.changeset, self.uid, self.user.to_string(), self.visible, self.tags.clone())
    }
}