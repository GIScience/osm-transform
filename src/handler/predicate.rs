use osm_io::osm::model::tag::Tag;
use rustc_hash::FxHashMap;

pub(crate) struct HasOneOfTagKeysPredicate {
    pub keys: Vec<String>
}
impl HasOneOfTagKeysPredicate {
    pub(crate) fn test(&mut self, tags: &Vec<Tag>) -> bool {
        tags.iter().any(|tag| self.keys.contains(tag.k()))
    }
}


pub(crate) struct HasTagKeyValuePredicate {
    pub key_values: FxHashMap<String,String>
}
impl HasTagKeyValuePredicate {
    pub(crate) fn test(&mut self, tags: &Vec<Tag>) -> bool {
        for tag in tags {
            if let Some(match_value) = self.key_values.get(tag.k()) {
                if tag.v() == match_value {
                    return true;
                }
            }
        }
        false
    }
}


pub(crate) struct HasNoneOfTagKeysPredicate {
    pub keys: Vec<String>
}
impl HasNoneOfTagKeysPredicate {
    pub(crate) fn test(&mut self, tags: &Vec<Tag>) -> bool {
        tags.iter().all(|tag| !self.keys.contains(tag.k()))
    }
}


#[cfg(test)]
mod test {
    use osm_io::osm::model::tag::Tag;
    use rustc_hash::FxHashMap;
    use crate::handler::predicate::{HasNoneOfTagKeysPredicate, HasOneOfTagKeysPredicate, HasTagKeyValuePredicate};

    #[test]
    fn has_one_of_tag_keys_predicate_with_only_matching_tags() {
        let mut predicate = HasOneOfTagKeysPredicate { keys: vec!["good".to_string(), "nice".to_string()] };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
        ]));
    }
    #[test]
    fn has_one_of_tag_keys_predicate_with_only_all_matching_tags() {
        let mut predicate = HasOneOfTagKeysPredicate { keys: vec!["good".to_string(), "nice".to_string()] };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
            Tag::new("nice".to_string(), "2".to_string()),
        ]));
    }
    #[test]
    fn has_one_of_tag_keys_predicate_with_also_matching_tags() {
        let mut predicate = HasOneOfTagKeysPredicate { keys: vec!["good".to_string(), "nice".to_string()] };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
            Tag::new("bad".to_string(), "2".to_string()),
        ]));
    }
    #[test]
    fn has_one_of_tag_keys_predicate_with_no_matching_tags() {
        let mut predicate = HasOneOfTagKeysPredicate { keys: vec!["good".to_string(), "nice".to_string()] };
        assert_eq!(false, predicate.test(&vec![
            Tag::new("ugly".to_string(), "1".to_string()),
            Tag::new("bad".to_string(), "2".to_string()),
        ]));
    }

    #[test]
    fn has_tag_key_value_predicate_with_no_matching_tag() {
        let mut key_values = FxHashMap::default();
        key_values.insert("good".to_string(), "good".to_string());
        key_values.insert("nice".to_string(), "nice".to_string());
        let mut predicate = HasTagKeyValuePredicate { key_values };
        assert_eq!(false, predicate.test(&vec![
            Tag::new("bad".to_string(), "1".to_string()),
            Tag::new("ugly".to_string(), "1".to_string()),
        ]));
    }
    #[test]
    fn has_tag_key_value_predicate_with_only_tag_with_wrong_value() {
        let mut key_values = FxHashMap::default();
        key_values.insert("good".to_string(), "good".to_string());
        key_values.insert("nice".to_string(), "nice".to_string());
        let mut predicate = HasTagKeyValuePredicate { key_values };
        assert_eq!(false, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
        ]));
    }
    #[test]
    fn has_tag_key_value_predicate_with_also_tag_with_wrong_value() {
        let mut key_values = FxHashMap::default();
        key_values.insert("good".to_string(), "good".to_string());
        key_values.insert("nice".to_string(), "nice".to_string());
        let mut predicate = HasTagKeyValuePredicate { key_values };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("bad".to_string(), "1".to_string()),
            Tag::new("good".to_string(), "1".to_string()),
            Tag::new("nice".to_string(), "nice".to_string()),
        ]));
    }
    #[test]
    fn has_tag_key_value_predicate_with_only_tag_with_matching_value() {
        let mut key_values = FxHashMap::default();
        key_values.insert("good".to_string(), "good".to_string());
        key_values.insert("nice".to_string(), "nice".to_string());
        let mut predicate = HasTagKeyValuePredicate { key_values };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "good".to_string()),
        ]));
    }
    #[test]
    fn has_tag_key_value_predicate_with_also_tag_with_matching_value() {
        let mut key_values = FxHashMap::default();
        key_values.insert("good".to_string(), "good".to_string());
        key_values.insert("nice".to_string(), "nice".to_string());
        let mut predicate = HasTagKeyValuePredicate { key_values };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("bad".to_string(), "1".to_string()),
            Tag::new("good".to_string(), "good".to_string()),
        ]));
    }

    #[test]
    fn has_none_of_tag_keys_predicate_with_only_non_matching_tag() {
        let mut predicate = HasNoneOfTagKeysPredicate { keys: vec!["bad".to_string(), "ugly".to_string()] };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
        ]));
    }
    #[test]
    fn has_none_of_tag_keys_predicate_also_matching_tag() {
        let mut predicate = HasNoneOfTagKeysPredicate { keys: vec!["bad".to_string(), "ugly".to_string()] };
        assert_eq!(false, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
            Tag::new("bad".to_string(), "1".to_string()),
        ]));
    }
    #[test]
    fn has_none_of_tag_keys_predicate_only_matching_tags() {
        let mut predicate = HasNoneOfTagKeysPredicate { keys: vec!["bad".to_string(), "ugly".to_string()] };
        assert_eq!(false, predicate.test(&vec![
            Tag::new("ugly".to_string(), "1".to_string()),
            Tag::new("bad".to_string(), "1".to_string()),
        ]));
    }

}