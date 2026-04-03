#![allow(missing_docs, dead_code)]

// Call scheduling and calendar module
// nexus-relay/src/scheduling.rs

use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum ScheduleType {
    OneTime,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum ReminderType {
    OnTime,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    OneDay,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ScheduledCall {
    pub id: String,
    pub initiator_id: String,
    pub recipient_id: String,
    pub scheduled_at: DateTime<Utc>,
    pub duration_minutes: u32,
    pub schedule_type: ScheduleType,
    pub title: String,
    pub description: Option<String>,
    pub timezone: String,
    pub reminders: Vec<ReminderType>,
    pub is_video: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CallReminder {
    pub id: String,
    pub scheduled_call_id: String,
    pub user_id: String,
    pub reminder_time: DateTime<Utc>,
    pub reminder_type: ReminderType,
    pub sent: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Availability {
    pub user_id: String,
    pub day_of_week: String, // 0-6, Monday-Sunday
    pub start_hour: u8,
    pub end_hour: u8,
    pub timezone: String,
}

pub(crate) struct SchedulingService {
    scheduled_calls: HashMap<String, ScheduledCall>,
    reminders: HashMap<String, CallReminder>,
    availability: HashMap<String, Vec<Availability>>,
}

impl SchedulingService {
    pub(crate) fn new() -> Self {
        SchedulingService {
            scheduled_calls: HashMap::new(),
            reminders: HashMap::new(),
            availability: HashMap::new(),
        }
    }

    pub(crate) fn create_scheduled_call(
        &mut self,
        initiator_id: &str,
        recipient_id: &str,
        scheduled_at: DateTime<Utc>,
        duration_minutes: u32,
        title: &str,
        is_video: bool,
    ) -> Result<ScheduledCall, String> {
        if scheduled_at < Utc::now() {
            return Err("Cannot schedule call in the past".to_string());
        }

        let call = ScheduledCall {
            id: format!("call_{}", uuid::Uuid::new_v4()),
            initiator_id: initiator_id.to_string(),
            recipient_id: recipient_id.to_string(),
            scheduled_at,
            duration_minutes,
            schedule_type: ScheduleType::OneTime,
            title: title.to_string(),
            description: None,
            timezone: "UTC".to_string(),
            reminders: vec![ReminderType::FifteenMinutes, ReminderType::OnTime],
            is_video,
        };

        self.scheduled_calls.insert(call.id.clone(), call.clone());
        
        // Create reminders
        self.create_reminders(&call.id, &call.recipient_id, call.scheduled_at, &call.reminders)?;

        Ok(call)
    }

    pub(crate) fn create_recurring_call(
        &mut self,
        initiator_id: &str,
        recipient_id: &str,
        schedule_type: ScheduleType,
        start_time: DateTime<Utc>,
        duration_minutes: u32,
        title: &str,
        is_video: bool,
    ) -> Result<ScheduledCall, String> {
        let call = ScheduledCall {
            id: format!("call_{}", uuid::Uuid::new_v4()),
            initiator_id: initiator_id.to_string(),
            recipient_id: recipient_id.to_string(),
            scheduled_at: start_time,
            duration_minutes,
            schedule_type,
            title: title.to_string(),
            description: None,
            timezone: "UTC".to_string(),
            reminders: vec![ReminderType::FifteenMinutes],
            is_video,
        };

        self.scheduled_calls.insert(call.id.clone(), call.clone());
        Ok(call)
    }

    pub(crate) fn set_availability(
        &mut self,
        user_id: &str,
        availabilities: Vec<Availability>,
    ) -> Result<(), String> {
        for availability in &availabilities {
            if availability.start_hour >= availability.end_hour || availability.end_hour > 24 {
                return Err("Invalid availability window".to_string());
            }
        }

        self.availability
            .insert(user_id.to_string(), availabilities);
        Ok(())
    }

    pub(crate) fn find_available_slots(
        &self,
        user_id: &str,
        duration_minutes: u32,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<DateTime<Utc>>, String> {
        let availabilities = self
            .availability
            .get(user_id)
            .ok_or("No availability set")?;

        let mut available_slots = Vec::new();
        let mut current = from;

        while current < to {
            // Check if there's an availability slot for this day
            let day_of_week = current.format("%w").to_string();
            
            if let Some(_avail) = availabilities.iter().find(|a| a.day_of_week == day_of_week) {
                // Check if slot is not already booked
                let is_booked = self.scheduled_calls.values().any(|call| {
                    (call.recipient_id == user_id || call.initiator_id == user_id)
                        && call.scheduled_at <= current
                        && call.scheduled_at
                            + Duration::minutes(call.duration_minutes as i64)
                            >= current + Duration::minutes(duration_minutes as i64)
                });

                if !is_booked {
                    available_slots.push(current);
                }
            }

            current = current + Duration::minutes(30);
        }

        Ok(available_slots)
    }

    pub(crate) fn get_upcoming_calls(&self, user_id: &str) -> Vec<&ScheduledCall> {
        let now = Utc::now();
        self.scheduled_calls
            .values()
            .filter(|call| {
                (call.recipient_id == user_id || call.initiator_id == user_id)
                    && call.scheduled_at > now
            })
            .collect()
    }

    fn create_reminders(&mut self, call_id: &str, recipient_id: &str, scheduled_at: DateTime<Utc>, reminders: &[ReminderType]) -> Result<(), String> {
        for reminder_type in reminders {
            let reminder_time = match reminder_type {
                ReminderType::OnTime => scheduled_at,
                ReminderType::FiveMinutes => scheduled_at - Duration::minutes(5),
                ReminderType::FifteenMinutes => scheduled_at - Duration::minutes(15),
                ReminderType::OneHour => scheduled_at - Duration::minutes(60),
                ReminderType::OneDay => scheduled_at - Duration::days(1),
            };

            let reminder = CallReminder {
                id: format!("reminder_{}", uuid::Uuid::new_v4()),
                scheduled_call_id: call_id.to_string(),
                user_id: recipient_id.to_string(),
                reminder_time,
                reminder_type: reminder_type.clone(),
                sent: false,
            };

            self.reminders.insert(reminder.id.clone(), reminder);
        }

        Ok(())
    }

    pub(crate) fn cancel_scheduled_call(&mut self, call_id: &str) -> Result<(), String> {
        self.scheduled_calls.remove(call_id).ok_or("Call not found")?;
        
        // Remove associated reminders
        self.reminders
            .retain(|_, r| r.scheduled_call_id != call_id);

        Ok(())
    }

    pub(crate) fn reschedule_call(
        &mut self,
        call_id: &str,
        new_time: DateTime<Utc>,
    ) -> Result<(), String> {
        if let Some(call) = self.scheduled_calls.get_mut(call_id) {
            if new_time < Utc::now() {
                return Err("Cannot reschedule to past time".to_string());
            }
            call.scheduled_at = new_time;

            // Remove old reminders
            self.reminders
                .retain(|_, r| r.scheduled_call_id != call_id);

            // Recreate reminders with new time
            let call_id = call.id.clone();
            let recipient_id = call.recipient_id.clone();
            let reminders = call.reminders.clone();
            
            self.create_reminders(&call_id, &recipient_id, new_time, &reminders)?;

            Ok(())
        } else {
            Err("Call not found".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_scheduled_call() {
        let mut service = SchedulingService::new();
        let future = Utc::now() + Duration::hours(24);
        
        let result = service.create_scheduled_call(
            "user_1",
            "user_2",
            future,
            60,
            "Team Meeting",
            true,
        );

        assert!(result.is_ok());
        let call = result.unwrap();
        assert_eq!(call.title, "Team Meeting");
        assert!(call.is_video);
    }

    #[test]
    fn test_cannot_schedule_past_time() {
        let mut service = SchedulingService::new();
        let past = Utc::now() - Duration::hours(1);

        let result = service.create_scheduled_call("user_1", "user_2", past, 60, "Meeting", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_recurring_call() {
        let mut service = SchedulingService::new();
        let future = Utc::now() + Duration::hours(24);

        let result = service.create_recurring_call(
            "user_1",
            "user_2",
            ScheduleType::Weekly,
            future,
            60,
            "Weekly Sync",
            false,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap().schedule_type, ScheduleType::Weekly);
    }

    #[test]
    fn test_set_availability() {
        let mut service = SchedulingService::new();
        let availabilities = vec![Availability {
            user_id: "user_1".to_string(),
            day_of_week: "1".to_string(),
            start_hour: 9,
            end_hour: 17,
            timezone: "UTC".to_string(),
        }];

        let result = service.set_availability("user_1", availabilities);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_upcoming_calls() {
        let mut service = SchedulingService::new();
        let future = Utc::now() + Duration::hours(24);

        service
            .create_scheduled_call("user_1", "user_2", future, 60, "Meeting", true)
            .unwrap();

        let upcoming = service.get_upcoming_calls("user_1");
        assert_eq!(upcoming.len(), 1);
    }

    #[test]
    fn test_cancel_scheduled_call() {
        let mut service = SchedulingService::new();
        let future = Utc::now() + Duration::hours(24);

        let call = service
            .create_scheduled_call("user_1", "user_2", future, 60, "Meeting", true)
            .unwrap();

        let result = service.cancel_scheduled_call(&call.id);
        assert!(result.is_ok());
        assert!(service.get_upcoming_calls("user_1").is_empty());
    }

    #[test]
    fn test_reschedule_call() {
        let mut service = SchedulingService::new();
        let future_1 = Utc::now() + Duration::hours(24);
        let future_2 = Utc::now() + Duration::hours(48);

        let call = service
            .create_scheduled_call("user_1", "user_2", future_1, 60, "Meeting", true)
            .unwrap();

        let result = service.reschedule_call(&call.id, future_2);
        assert!(result.is_ok());
    }
}
