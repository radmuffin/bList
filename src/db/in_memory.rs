use chrono::Utc;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::RwLock;

use super::{ListRepository, PinRepository, StorageError, UserRepository};
use crate::models::{
    Collaborator, CreateListRequest, CreatePinRequest, List, ListPinsQuery, Pin, UpdateListRequest,
    UpdatePinRequest, UpdateUserProfileRequest, UserProfile,
};

/// In-Memory Storage Engine (For Unit Testing & Ephemeral Deployments)
#[allow(dead_code)]
#[derive(Default)]
pub struct InMemoryStorage {
    lists: RwLock<HashMap<i64, List>>,
    pins: RwLock<HashMap<i64, Pin>>,
    device_lists: RwLock<Vec<(String, i64)>>,
    users: RwLock<HashMap<String, UserProfile>>,
    next_list_id: RwLock<i64>,
    next_pin_id: RwLock<i64>,
}

impl InMemoryStorage {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let storage = Self::default();
        let default_list = List {
            id: 1,
            name: "My Bucket List".to_string(),
            icon: "📍".to_string(),
            created_at: Utc::now().to_rfc3339(),
            owner_token: "".to_string(),
            share_token: uuid::Uuid::new_v4().to_string(),
        };
        storage.lists.write().unwrap().insert(1, default_list);
        *storage.next_list_id.write().unwrap() = 2;
        *storage.next_pin_id.write().unwrap() = 1;
        storage
    }
}

impl ListRepository for InMemoryStorage {
    fn list_lists(&self, user_token: &str) -> Result<Vec<List>, StorageError> {
        self.auto_associate_device(user_token)?;

        let device_lists = self.device_lists.read().unwrap();
        let lists = self.lists.read().unwrap();
        let mut result = Vec::new();
        for (tok, lid) in device_lists.iter() {
            if tok == user_token {
                if let Some(list) = lists.get(lid) {
                    result.push(list.clone());
                }
            }
        }
        result.sort_by_key(|l| l.id);
        Ok(result)
    }

    fn get_list(&self, id: i64) -> Result<Option<List>, StorageError> {
        let lists = self.lists.read().unwrap();
        Ok(lists.get(&id).cloned())
    }

    fn create_list(&self, req: &CreateListRequest, user_token: &str) -> Result<List, StorageError> {
        let mut next_id = self.next_list_id.write().unwrap();
        let id = *next_id;
        *next_id += 1;

        let icon = match &req.icon {
            Some(i) if !i.trim().is_empty() => i.trim().to_string(),
            _ => "📍".to_string(),
        };

        let list = List {
            id,
            name: req.name.trim().to_string(),
            icon,
            created_at: Utc::now().to_rfc3339(),
            owner_token: user_token.to_string(),
            share_token: uuid::Uuid::new_v4().to_string(),
        };

        self.lists.write().unwrap().insert(id, list.clone());

        // Auto-associate the new list
        self.device_lists
            .write()
            .unwrap()
            .push((user_token.to_string(), id));

        Ok(list)
    }

    fn update_list(&self, id: i64, req: &UpdateListRequest) -> Result<Option<List>, StorageError> {
        let mut lists = self.lists.write().unwrap();
        if let Some(existing) = lists.get_mut(&id) {
            if let Some(ref name) = req.name {
                if !name.trim().is_empty() {
                    existing.name = name.trim().to_string();
                }
            }
            if let Some(ref icon) = req.icon {
                if !icon.trim().is_empty() {
                    existing.icon = icon.trim().to_string();
                }
            }
            Ok(Some(existing.clone()))
        } else {
            Ok(None)
        }
    }

    fn delete_list(&self, id: i64) -> Result<bool, StorageError> {
        let mut lists = self.lists.write().unwrap();
        let removed = lists.remove(&id).is_some();
        if removed {
            let mut pins = self.pins.write().unwrap();
            pins.retain(|_, p| p.list_id != id);

            // Remove device list mappings
            let mut device_lists = self.device_lists.write().unwrap();
            device_lists.retain(|(_, lid)| *lid != id);
        }
        Ok(removed)
    }

    fn check_permission(&self, user_token: &str, list_id: i64) -> Result<bool, StorageError> {
        self.auto_associate_device(user_token)?;
        let device_lists = self.device_lists.read().unwrap();
        Ok(device_lists
            .iter()
            .any(|(tok, lid)| tok == user_token && *lid == list_id))
    }

    fn join_list(&self, share_token: &str, user_token: &str) -> Result<Option<List>, StorageError> {
        self.auto_associate_device(user_token)?;
        let lists = self.lists.read().unwrap();
        if let Some(list) = lists.values().find(|l| l.share_token == share_token) {
            let mut device_lists = self.device_lists.write().unwrap();
            if !device_lists
                .iter()
                .any(|(tok, lid)| tok == user_token && *lid == list.id)
            {
                device_lists.push((user_token.to_string(), list.id));
            }
            Ok(Some(list.clone()))
        } else {
            Ok(None)
        }
    }

    fn auto_associate_device(&self, user_token: &str) -> Result<(), StorageError> {
        let mut device_lists = self.device_lists.write().unwrap();
        let has_lists = device_lists.iter().any(|(tok, _)| tok == user_token);
        if !has_lists {
            let mut lists = self.lists.write().unwrap();
            let list1_claimed = device_lists.iter().any(|(_, lid)| *lid == 1);
            if let Some(list1) = lists.get_mut(&1) {
                if list1.owner_token.is_empty() && !list1_claimed {
                    list1.owner_token = user_token.to_string();
                    device_lists.push((user_token.to_string(), 1));
                    return Ok(());
                }
            }
            let mut next_id = self.next_list_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            let list = List {
                id,
                name: "My Bucket List".to_string(),
                icon: "📍".to_string(),
                created_at: Utc::now().to_rfc3339(),
                owner_token: user_token.to_string(),
                share_token: uuid::Uuid::new_v4().to_string(),
            };
            lists.insert(id, list);
            device_lists.push((user_token.to_string(), id));
        }
        Ok(())
    }

    fn count_user_lists(&self, user_token: &str) -> Result<usize, StorageError> {
        self.auto_associate_device(user_token)?;
        let device_lists = self.device_lists.read().unwrap();
        let count = device_lists
            .iter()
            .filter(|(tok, _)| tok == user_token)
            .count();
        Ok(count)
    }
}

impl PinRepository for InMemoryStorage {
    fn list_pins(&self, query: &ListPinsQuery, user_token: &str) -> Result<Vec<Pin>, StorageError> {
        self.auto_associate_device(user_token)?;
        let device_lists = self.device_lists.read().unwrap();
        let allowed_lists: HashSet<i64> = device_lists
            .iter()
            .filter(|(tok, _)| tok == user_token)
            .map(|(_, lid)| *lid)
            .collect();

        let pins = self.pins.read().unwrap();
        let mut result: Vec<Pin> = pins
            .values()
            .filter(|p| {
                if !allowed_lists.contains(&p.list_id) {
                    return false;
                }
                if let Some(lid) = query.list_id {
                    if p.list_id != lid {
                        return false;
                    }
                }
                if let Some(ref cat) = query.category {
                    if !cat.is_empty() && cat != "All" && &p.category != cat {
                        return false;
                    }
                }
                if let Some(visited) = query.visited {
                    if p.visited != visited {
                        return false;
                    }
                }
                if let Some(priority) = query.priority {
                    if p.priority != priority {
                        return false;
                    }
                }
                if let Some(day) = query.day_group {
                    if p.day_group != day {
                        return false;
                    }
                }
                if let Some(ref tag) = query.tag {
                    let trimmed = tag.trim().trim_start_matches('#');
                    if !trimmed.is_empty() {
                        let has_tag = p.tags.as_deref().unwrap_or("").contains(trimmed);
                        if !has_tag {
                            return false;
                        }
                    }
                }
                if let Some(ref search) = query.search {
                    let s = search.trim().to_lowercase();
                    if !s.is_empty() {
                        let title_m = p.title.to_lowercase().contains(&s);
                        let desc_m = p
                            .description
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&s);
                        let addr_m = p
                            .address
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&s);
                        let notes_m = p.notes.as_deref().unwrap_or("").to_lowercase().contains(&s);
                        let tags_m = p.tags.as_deref().unwrap_or("").to_lowercase().contains(&s);
                        if !title_m && !desc_m && !addr_m && !notes_m && !tags_m {
                            return false;
                        }
                    }
                }
                true
            })
            .cloned()
            .collect();

        result.sort_by_key(|a| (a.custom_order, std::cmp::Reverse(a.id)));
        Ok(result)
    }

    fn get_pin(&self, id: i64) -> Result<Option<Pin>, StorageError> {
        let pins = self.pins.read().unwrap();
        Ok(pins.get(&id).cloned())
    }

    fn find_duplicate_pin(
        &self,
        list_id: i64,
        title: &str,
        lat: f64,
        lon: f64,
        source_url: Option<&str>,
        exclude_id: Option<i64>,
    ) -> Result<Option<Pin>, StorageError> {
        let clean_title = title.trim().to_lowercase();
        let clean_source = source_url
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());

        let pins = self.pins.read().unwrap();
        for p in pins.values() {
            if p.list_id != list_id {
                continue;
            }
            if let Some(eid) = exclude_id {
                if p.id == eid {
                    continue;
                }
            }
            if let Some(ref src) = clean_source {
                if let Some(ref p_src) = p.source_url {
                    if p_src.trim().to_lowercase() == *src {
                        return Ok(Some(p.clone()));
                    }
                }
            }
            let lat_diff = (p.latitude - lat).abs();
            let lon_diff = (p.longitude - lon).abs();
            if lat_diff < 0.0001 && lon_diff < 0.0001 {
                return Ok(Some(p.clone()));
            }
            if p.title.trim().to_lowercase() == clean_title && lat_diff < 0.001 && lon_diff < 0.001
            {
                return Ok(Some(p.clone()));
            }
        }
        Ok(None)
    }

    fn create_pin(&self, req: &CreatePinRequest) -> Result<Pin, StorageError> {
        let mut next_id = self.next_pin_id.write().unwrap();
        let id = *next_id;
        *next_id += 1;

        let pin = Pin {
            id,
            list_id: req.list_id.unwrap_or(1),
            title: req.title.clone(),
            description: req.description.clone(),
            latitude: req.latitude,
            longitude: req.longitude,
            category: req
                .category
                .clone()
                .unwrap_or_else(|| "General".to_string()),
            emoji: req.emoji.clone(),
            tags: req.tags.clone(),
            priority: req.priority.unwrap_or(false),
            day_group: req.day_group.unwrap_or(0),
            custom_order: req.custom_order.unwrap_or(0),
            opening_hours: req.opening_hours.clone(),
            source_url: req.source_url.clone(),
            image_url: req.image_url.clone(),
            address: req.address.clone(),
            notes: req.notes.clone(),
            visited: req.visited.unwrap_or(false),
            created_at: Utc::now().to_rfc3339(),
        };

        self.pins.write().unwrap().insert(id, pin.clone());
        Ok(pin)
    }

    fn create_pins_batch(
        &self,
        list_id: i64,
        pins: &[CreatePinRequest],
    ) -> Result<Vec<Pin>, StorageError> {
        let mut created = Vec::with_capacity(pins.len());
        for req in pins {
            let mut req_copy = req.clone();
            req_copy.list_id = Some(list_id);
            created.push(self.create_pin(&req_copy)?);
        }
        Ok(created)
    }

    fn update_pin(&self, id: i64, req: &UpdatePinRequest) -> Result<Option<Pin>, StorageError> {
        let mut pins = self.pins.write().unwrap();
        if let Some(pin) = pins.get_mut(&id) {
            if let Some(lid) = req.list_id {
                pin.list_id = lid;
            }
            if let Some(ref t) = req.title {
                pin.title = t.clone();
            }
            if req.description.is_some() {
                pin.description = req.description.clone();
            }
            if let Some(lat) = req.latitude {
                pin.latitude = lat;
            }
            if let Some(lon) = req.longitude {
                pin.longitude = lon;
            }
            if let Some(ref cat) = req.category {
                pin.category = cat.clone();
            }
            if let Some(ref e) = req.emoji {
                pin.emoji = Some(e.clone());
            }
            if let Some(ref t) = req.tags {
                pin.tags = Some(t.clone());
            }
            if let Some(p) = req.priority {
                pin.priority = p;
            }
            if let Some(d) = req.day_group {
                pin.day_group = d;
            }
            if let Some(c) = req.custom_order {
                pin.custom_order = c;
            }
            if req.opening_hours.is_some() {
                pin.opening_hours = req.opening_hours.clone();
            }
            if req.source_url.is_some() {
                pin.source_url = req.source_url.clone();
            }
            if req.image_url.is_some() {
                pin.image_url = req.image_url.clone();
            }
            if req.address.is_some() {
                pin.address = req.address.clone();
            }
            if req.notes.is_some() {
                pin.notes = req.notes.clone();
            }
            if let Some(v) = req.visited {
                pin.visited = v;
            }
            Ok(Some(pin.clone()))
        } else {
            Ok(None)
        }
    }

    fn toggle_visited(&self, id: i64) -> Result<Option<Pin>, StorageError> {
        let mut pins = self.pins.write().unwrap();
        if let Some(pin) = pins.get_mut(&id) {
            pin.visited = !pin.visited;
            Ok(Some(pin.clone()))
        } else {
            Ok(None)
        }
    }

    fn delete_pin(&self, id: i64) -> Result<bool, StorageError> {
        let mut pins = self.pins.write().unwrap();
        Ok(pins.remove(&id).is_some())
    }

    fn get_categories(
        &self,
        list_id: Option<i64>,
        user_token: &str,
    ) -> Result<Vec<String>, StorageError> {
        self.auto_associate_device(user_token)?;
        let device_lists = self.device_lists.read().unwrap();
        let allowed_lists: HashSet<i64> = device_lists
            .iter()
            .filter(|(tok, _)| tok == user_token)
            .map(|(_, lid)| *lid)
            .collect();

        let pins = self.pins.read().unwrap();
        let mut set = BTreeSet::new();
        for pin in pins.values() {
            if !allowed_lists.contains(&pin.list_id) {
                continue;
            }
            if let Some(lid) = list_id {
                if pin.list_id != lid {
                    continue;
                }
            }
            if !pin.category.trim().is_empty() {
                set.insert(pin.category.clone());
            }
        }
        Ok(set.into_iter().collect())
    }

    fn count_list_pins(&self, list_id: i64) -> Result<usize, StorageError> {
        let pins = self.pins.read().unwrap();
        let count = pins.values().filter(|p| p.list_id == list_id).count();
        Ok(count)
    }

    fn count_user_pins(&self, user_token: &str) -> Result<usize, StorageError> {
        self.auto_associate_device(user_token)?;
        let device_lists = self.device_lists.read().unwrap();
        let allowed_lists: HashSet<i64> = device_lists
            .iter()
            .filter(|(tok, _)| tok == user_token)
            .map(|(_, lid)| *lid)
            .collect();
        let pins = self.pins.read().unwrap();
        let count = pins
            .values()
            .filter(|p| allowed_lists.contains(&p.list_id))
            .count();
        Ok(count)
    }
}

impl UserRepository for InMemoryStorage {
    fn get_user_profile(&self, user_token: &str) -> Result<UserProfile, StorageError> {
        let users = self.users.read().unwrap();
        if let Some(profile) = users.get(user_token) {
            Ok(profile.clone())
        } else {
            Ok(UserProfile {
                user_token: user_token.to_string(),
                name: "".to_string(),
                avatar: "🧭".to_string(),
                color: "#3b82f6".to_string(),
            })
        }
    }

    fn update_user_profile(
        &self,
        user_token: &str,
        req: &UpdateUserProfileRequest,
    ) -> Result<UserProfile, StorageError> {
        let mut users = self.users.write().unwrap();
        let mut profile = users
            .get(user_token)
            .cloned()
            .unwrap_or_else(|| UserProfile {
                user_token: user_token.to_string(),
                name: "".to_string(),
                avatar: "🧭".to_string(),
                color: "#3b82f6".to_string(),
            });

        if let Some(ref name) = req.name {
            profile.name = name.trim().to_string();
        }
        if let Some(ref avatar) = req.avatar {
            profile.avatar = avatar.trim().to_string();
        }
        if let Some(ref color) = req.color {
            profile.color = color.trim().to_string();
        }

        users.insert(user_token.to_string(), profile.clone());
        Ok(profile)
    }

    fn get_list_collaborators(&self, list_id: i64) -> Result<Vec<Collaborator>, StorageError> {
        let device_lists = self.device_lists.read().unwrap();
        let users = self.users.read().unwrap();
        let lists = self.lists.read().unwrap();

        let list_owner_token = lists
            .get(&list_id)
            .map(|l| l.owner_token.as_str())
            .unwrap_or("");

        let mut collaborators = Vec::new();
        for (tok, lid) in device_lists.iter() {
            if *lid == list_id {
                let (name, avatar, color) = if let Some(u) = users.get(tok) {
                    (
                        if u.name.trim().is_empty() {
                            "Traveler".to_string()
                        } else {
                            u.name.clone()
                        },
                        if u.avatar.trim().is_empty() {
                            "🧭".to_string()
                        } else {
                            u.avatar.clone()
                        },
                        if u.color.trim().is_empty() {
                            "#3b82f6".to_string()
                        } else {
                            u.color.clone()
                        },
                    )
                } else {
                    (
                        "Traveler".to_string(),
                        "🧭".to_string(),
                        "#3b82f6".to_string(),
                    )
                };

                let is_owner = !list_owner_token.is_empty() && list_owner_token == tok;
                collaborators.push(Collaborator {
                    name,
                    avatar,
                    color,
                    is_owner,
                });
            }
        }

        Ok(collaborators)
    }
}
