import Foundation
import UserNotifications
import UIKit

// MARK: - Notification Types

enum NexusNotificationType: String {
    case newMessage = "new_message"
    case incomingCall = "incoming_call"
    case groupInvite = "group_invite"
    case securityAlert = "security_alert"
    case contactRequest = "contact_request"
}

// MARK: - Notification Content

struct NexusNotification {
    let type: NexusNotificationType
    let title: String
    let body: String
    let senderHash: String?
    let conversationId: String?
    let callId: String?
    let groupId: String?
    let timestamp: Date
    let isEncrypted: Bool
}

// MARK: - Notification Manager

final class NotificationManager: NSObject {
    static let shared = NotificationManager()

    private let center = UNUserNotificationCenter.current()
    private var isAuthorized = false

    // Callbacks
    var onNotificationReceived: ((NexusNotification) -> Void)?
    var onNotificationTapped: ((NexusNotification) -> Void)?

    private override init() {
        super.init()
        center.delegate = self
    }

    // MARK: - Authorization

    func requestAuthorization() async -> Bool {
        do {
            let granted = try await center.requestAuthorization(
                options: [.alert, .badge, .sound, .providesAppNotificationSettings]
            )
            isAuthorized = granted

            if granted {
                await registerNotificationCategories()
            }

            return granted
        } catch {
            print("Notification authorization error: \(error)")
            return false
        }
    }

    func checkAuthorizationStatus() async -> UNAuthorizationStatus {
        let settings = await center.notificationSettings()
        return settings.authorizationStatus
    }

    // MARK: - Register Categories

    private func registerNotificationCategories() async {
        // Message actions
        let replyAction = UNTextInputNotificationAction(
            identifier: "REPLY_ACTION",
            title: "Reply",
            options: [],
            textInputButtonTitle: "Send",
            textInputPlaceholder: "Type a message..."
        )

        let markReadAction = UNNotificationAction(
            identifier: "MARK_READ_ACTION",
            title: "Mark as Read",
            options: .destructive
        )

        let messageCategory = UNNotificationCategory(
            identifier: "MESSAGE_CATEGORY",
            actions: [replyAction, markReadAction],
            intentIdentifiers: [],
            options: .customDismissAction
        )

        // Call actions
        let answerAction = UNNotificationAction(
            identifier: "ANSWER_CALL_ACTION",
            title: "Answer",
            options: .foreground
        )

        let declineAction = UNNotificationAction(
            identifier: "DECLINE_CALL_ACTION",
            title: "Decline",
            options: .destructive
        )

        let callCategory = UNNotificationCategory(
            identifier: "CALL_CATEGORY",
            actions: [answerAction, declineAction],
            intentIdentifiers: [],
            options: []
        )

        // Group actions
        let joinAction = UNNotificationAction(
            identifier: "JOIN_GROUP_ACTION",
            title: "Join",
            options: .foreground
        )

        let declineGroupAction = UNNotificationAction(
            identifier: "DECLINE_GROUP_ACTION",
            title: "Decline",
            options: .destructive
        )

        let groupCategory = UNNotificationCategory(
            identifier: "GROUP_CATEGORY",
            actions: [joinAction, declineGroupAction],
            intentIdentifiers: [],
            options: []
        )

        center.setNotificationCategories([messageCategory, callCategory, groupCategory])
    }

    // MARK: - Schedule Notifications

    func scheduleMessageNotification(
        senderName: String,
        messagePreview: String,
        senderHash: String,
        conversationId: String,
        isEncrypted: Bool = true
    ) async {
        guard isAuthorized else { return }

        let content = UNMutableNotificationContent()
        content.title = senderName
        content.body = isEncrypted ? "🔒 Encrypted message" : messagePreview
        content.sound = .default
        content.categoryIdentifier = "MESSAGE_CATEGORY"
        content.threadIdentifier = conversationId
        content.userInfo = [
            "type": NexusNotificationType.newMessage.rawValue,
            "senderHash": senderHash,
            "conversationId": conversationId,
            "isEncrypted": isEncrypted
        ]

        // Add badge
        content.badge = NSNumber(value: await getUnreadCount() + 1)

        let request = UNNotificationRequest(
            identifier: "msg-\(conversationId)-\(Date().timeIntervalSince1970)",
            content: content,
            trigger: nil // Immediate
        )

        do {
            try await center.add(request)
        } catch {
            print("Failed to schedule message notification: \(error)")
        }
    }

    func scheduleCallNotification(
        callerName: String,
        callType: String,
        callerHash: String,
        callId: String
    ) async {
        guard isAuthorized else { return }

        let content = UNMutableNotificationContent()
        content.title = "Incoming \(callType) Call"
        content.body = "\(callerName) is calling..."
        content.sound = UNNotificationSound(named: UNNotificationSoundName("ringtone.caf"))
        content.categoryIdentifier = "CALL_CATEGORY"
        content.interruptionLevel = .timeSensitive
        content.userInfo = [
            "type": NexusNotificationType.incomingCall.rawValue,
            "callerHash": callerHash,
            "callId": callId,
            "callType": callType
        ]

        let request = UNNotificationRequest(
            identifier: "call-\(callId)",
            content: content,
            trigger: nil
        )

        do {
            try await center.add(request)
        } catch {
            print("Failed to schedule call notification: \(error)")
        }
    }

    func scheduleGroupInviteNotification(
        groupName: String,
        inviterName: String,
        groupId: String
    ) async {
        guard isAuthorized else { return }

        let content = UNMutableNotificationContent()
        content.title = "Group Invitation"
        content.body = "\(inviterName) invited you to join \(groupName)"
        content.sound = .default
        content.categoryIdentifier = "GROUP_CATEGORY"
        content.userInfo = [
            "type": NexusNotificationType.groupInvite.rawValue,
            "groupId": groupId,
            "inviterName": inviterName
        ]

        let request = UNNotificationRequest(
            identifier: "group-invite-\(groupId)",
            content: content,
            trigger: nil
        )

        do {
            try await center.add(request)
        } catch {
            print("Failed to schedule group invite notification: \(error)")
        }
    }

    func scheduleSecurityAlertNotification(
        alertType: String,
        details: String
    ) async {
        guard isAuthorized else { return }

        let content = UNMutableNotificationContent()
        content.title = "Security Alert"
        content.body = details
        content.sound = .defaultCritical
        content.interruptionLevel = .critical
        content.userInfo = [
            "type": NexusNotificationType.securityAlert.rawValue,
            "alertType": alertType
        ]

        let request = UNNotificationRequest(
            identifier: "security-\(Date().timeIntervalSince1970)",
            content: content,
            trigger: nil
        )

        do {
            try await center.add(request)
        } catch {
            print("Failed to schedule security alert: \(error)")
        }
    }

    // MARK: - Cancel Notifications

    func cancelNotification(identifier: String) {
        center.removeDeliveredNotifications(withIdentifiers: [identifier])
        center.removePendingNotificationRequests(withIdentifiers: [identifier])
    }

    func cancelAllNotifications() {
        center.removeAllDeliveredNotifications()
        center.removeAllPendingNotificationRequests()
        clearBadge()
    }

    func cancelCallNotifications(callId: String) {
        cancelNotification(identifier: "call-\(callId)")
    }

    func cancelConversationNotifications(conversationId: String) {
        center.getDeliveredNotifications { notifications in
            let identifiers = notifications
                .filter { $0.request.content.threadIdentifier == conversationId }
                .map { $0.request.identifier }
            self.center.removeDeliveredNotifications(withIdentifiers: identifiers)
        }
    }

    // MARK: - Badge Management

    func updateBadge(count: Int) {
        DispatchQueue.main.async {
            UIApplication.shared.applicationIconBadgeNumber = count
        }
    }

    func clearBadge() {
        updateBadge(count: 0)
    }

    private func getUnreadCount() async -> Int {
        let delivered = await center.deliveredNotifications
        return delivered.count
    }

    // MARK: - Handle Actions

    func handleNotificationAction(
        identifier: String,
        notification: UNNotification
    ) {
        let userInfo = notification.request.content.userInfo

        guard let type = userInfo["type"] as? String,
              let notificationType = NexusNotificationType(rawValue: type) else {
            return
        }

        switch notificationType {
        case .newMessage:
            handleMessageAction(identifier: identifier, userInfo: userInfo)
        case .incomingCall:
            handleCallAction(identifier: identifier, userInfo: userInfo)
        case .groupInvite:
            handleGroupAction(identifier: identifier, userInfo: userInfo)
        case .securityAlert, .contactRequest:
            break
        }
    }

    private func handleMessageAction(identifier: String, userInfo: [AnyHashable: Any]) {
        switch identifier {
        case "REPLY_ACTION":
            // Handle reply - would open chat with pre-filled text
            break
        case "MARK_READ_ACTION":
            if let conversationId = userInfo["conversationId"] as? String {
                cancelConversationNotifications(conversationId: conversationId)
            }
        default:
            break
        }
    }

    private func handleCallAction(identifier: String, userInfo: [AnyHashable: Any]) {
        switch identifier {
        case "ANSWER_CALL_ACTION":
            // Answer call
            if let callId = userInfo["callId"] as? String {
                cancelCallNotifications(callId: callId)
            }
        case "DECLINE_CALL_ACTION":
            // Decline call
            if let callId = userInfo["callId"] as? String {
                cancelCallNotifications(callId: callId)
            }
        default:
            break
        }
    }

    private func handleGroupAction(identifier: String, userInfo: [AnyHashable: Any]) {
        switch identifier {
        case "JOIN_GROUP_ACTION":
            // Join group
            break
        case "DECLINE_GROUP_ACTION":
            // Decline group invite
            break
        default:
            break
        }
    }
}

// MARK: - UNUserNotificationCenterDelegate

extension NotificationManager: UNUserNotificationCenterDelegate {
    // Handle notification when app is in foreground
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        let userInfo = notification.request.content.userInfo

        // Create notification object
        if let type = userInfo["type"] as? String,
           let notificationType = NexusNotificationType(rawValue: type) {
            let nexusNotification = NexusNotification(
                type: notificationType,
                title: notification.request.content.title,
                body: notification.request.content.body,
                senderHash: userInfo["senderHash"] as? String,
                conversationId: userInfo["conversationId"] as? String,
                callId: userInfo["callId"] as? String,
                groupId: userInfo["groupId"] as? String,
                timestamp: notification.date,
                isEncrypted: userInfo["isEncrypted"] as? Bool ?? false
            )
            onNotificationReceived?(nexusNotification)
        }

        // Show notification even when app is in foreground
        return [.banner, .sound, .badge]
    }

    // Handle notification tap
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse
    ) async {
        let userInfo = response.notification.request.content.userInfo

        // Handle action
        if response.actionIdentifier != UNNotificationDefaultActionIdentifier &&
           response.actionIdentifier != UNNotificationDismissActionIdentifier {
            handleNotificationAction(
                identifier: response.actionIdentifier,
                notification: response.notification
            )
            return
        }

        // Handle tap
        if let type = userInfo["type"] as? String,
           let notificationType = NexusNotificationType(rawValue: type) {
            let nexusNotification = NexusNotification(
                type: notificationType,
                title: response.notification.request.content.title,
                body: response.notification.request.content.body,
                senderHash: userInfo["senderHash"] as? String,
                conversationId: userInfo["conversationId"] as? String,
                callId: userInfo["callId"] as? String,
                groupId: userInfo["groupId"] as? String,
                timestamp: response.notification.date,
                isEncrypted: userInfo["isEncrypted"] as? Bool ?? false
            )
            onNotificationTapped?(nexusNotification)
        }
    }
}
