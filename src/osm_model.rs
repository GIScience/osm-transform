use osm_io::osm::model::coordinate::Coordinate;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::tag::Tag;
use osm_io::osm::model::way::Way;

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


#[derive(Debug, Clone)]
pub struct MutableWay<'a> {
    way: &'a Way,
    id: Option<i64>,
    version: Option<i32>,
    timestamp: Option<i64>,
    changeset: Option<i64>,
    uid: Option<i32>,
    user: Option<String>,
    visible: Option<bool>,
    refs: Option<Vec<i64>>,
    tags: Option<Vec<Tag>>,
}

impl<'a> MutableWay<'a> {
    pub fn new(way: &'a Way) -> MutableWay {
        MutableWay {
            way: way,
            id: None,
            version: None,
            timestamp: None,
            changeset: None,
            uid: None,
            user: None,
            visible: None,
            refs: None,
            tags: None,
        }
    }

    fn set_id(mut self, id: i64) -> Self {
        self.id = Some(id);
        self
    }
    fn set_version(mut self, version: i32) -> Self {
        self.version = Some(version);
        self
    }
    fn set_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
    fn set_changeset(mut self, changeset: i64) -> Self {
        self.changeset = Some(changeset);
        self
    }
    fn set_uid(mut self, uid: i32) -> Self {
        self.uid = Some(uid);
        self
    }
    fn set_user(mut self, user: String) -> Self {
        self.user = Some(user);
        self
    }
    fn set_visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }
    fn set_refs(mut self, refs: Vec<i64>) -> Self {
        self.refs = Some(refs);
        self
    }
    fn set_tags(mut self, tags: Vec<Tag>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn build(self) -> Way {
        Way::new(
            self.id.unwrap_or(self.way.id()),
            self.version.unwrap_or(self.way.version()),
            self.timestamp.unwrap_or(self.way.timestamp()),
            self.changeset.unwrap_or(self.way.changeset()),
            self.uid.unwrap_or(self.way.uid()),
            self.user.unwrap_or_else(|| self.way.user().clone()),
            self.visible.unwrap_or(self.way.visible()),
            self.refs.unwrap_or(self.way.refs().clone()),
            self.tags.unwrap_or(self.way.tags().clone()))
    }
}


#[cfg(test)]
mod tests {
    use osm_io::osm::model::tag::Tag;
    use osm_io::osm::model::way::Way;

    use crate::osm_model::MutableWay;

    #[test]
    fn modifyable_way_no_field_changed() {
        let way = Way::new(1, 1, 1, 1, 1, "user".to_string(),
                           true, vec![1, 2], vec![Tag::new("a_key".to_string(), "a_value".to_string())]);
        let mutable_way = MutableWay::new(&way);
        let changed = mutable_way.build();
        dbg!(&changed);
        assert_eq!(changed.id(), 1);
        assert_eq!(changed.version(), 1);
        assert_eq!(changed.timestamp(), 1);
        assert_eq!(changed.changeset(), 1);
        assert_eq!(changed.uid(), 1);
        assert_eq!(changed.user(), "user");
        assert_eq!(changed.visible(), true);
        assert_eq!(*changed.refs(), vec![1, 2]);
        assert_eq!(*changed.tags(), vec![Tag::new("a_key".to_string(), "a_value".to_string())]);
    }
    #[test]
    fn modifyable_way_one_field_changed() {
        let way = Way::new(1, 1, 1, 1, 1, "user".to_string(),
                           true, vec![1, 2], vec![Tag::new("a_key".to_string(), "a_value".to_string())]);
        let mut mutable_way = MutableWay::new(&way);
        mutable_way = mutable_way.set_id(2);
        let changed = mutable_way.build();
        dbg!(&changed);
        assert_eq!(changed.id(), 2);
        assert_eq!(changed.version(), 1);
        assert_eq!(changed.timestamp(), 1);
        assert_eq!(changed.changeset(), 1);
        assert_eq!(changed.uid(), 1);
        assert_eq!(changed.user(), "user");
        assert_eq!(changed.visible(), true);
        assert_eq!(*changed.refs(), vec![1, 2]);
        assert_eq!(*changed.tags(), vec![Tag::new("a_key".to_string(), "a_value".to_string())]);
    }
    #[test]
    fn modifyable_way_all_fields_changed() {
        let way = Way::new(1, 1, 1, 1, 1, "user".to_string(),
                           true, vec![1, 2], vec![Tag::new("a_key".to_string(), "a_value".to_string())]);
        let mut mutable_way = MutableWay::new(&way);
        mutable_way = mutable_way.set_id(2);
        mutable_way = mutable_way.set_version(2);
        mutable_way = mutable_way.set_timestamp(2);
        mutable_way = mutable_way.set_changeset(2);
        mutable_way = mutable_way.set_uid(2);
        mutable_way = mutable_way.set_user("changed".to_string());
        mutable_way = mutable_way.set_visible(false);
        mutable_way = mutable_way.set_refs(vec![3,4]);
        mutable_way = mutable_way.set_tags(vec![Tag::new("new_key".to_string(), "new_value".to_string())]);
        let changed = mutable_way.build();
        dbg!(&changed);
        assert_eq!(changed.id(), 2);
        assert_eq!(changed.version(), 2);
        assert_eq!(changed.timestamp(), 2);
        assert_eq!(changed.changeset(), 2);
        assert_eq!(changed.uid(), 2);
        assert_eq!(changed.user(), "changed");
        assert_eq!(changed.visible(), false);
        assert_eq!(*changed.refs(), vec![3, 4]);
        assert_eq!(*changed.tags(), vec![Tag::new("new_key".to_string(), "new_value".to_string())]);    }

}