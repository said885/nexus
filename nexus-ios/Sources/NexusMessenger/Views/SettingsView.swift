// iOS Settings View - SwiftUI
// nexus-ios/Sources/NexusMessenger/Views/SettingsView.swift

import SwiftUI

struct SettingsView: View {
    @State private var notificationsEnabled = true
    @State private var darkModeEnabled = true
    @State private var endToEndEncryption = true
    @State private var showAbout = false
    @State private var showPrivacy = false
    
    var body: some View {
        NavigationStack {
            Form {
                // Account Section
                Section("Account") {
                    NavigationLink(destination: ProfileEditView()) {
                        Label("Profile", systemImage: "person.fill")
                    }
                    NavigationLink(destination: PasswordChangeView()) {
                        Label("Change Password", systemImage: "lock.fill")
                    }
                    NavigationLink(destination: TwoFactorView()) {
                        Label("Two-Factor Authentication", systemImage: "shield.fill")
                    }
                }
                
                // Privacy & Security
                Section("Privacy & Security") {
                    Toggle("End-to-End Encryption", isOn: $endToEndEncryption)
                    NavigationLink(destination: MessagePrivacyView()) {
                        Label("Message Privacy", systemImage: "message.circle.fill")
                    }
                    NavigationLink(destination: VisibilityView()) {
                        Label("Visibility", systemImage: "eye.fill")
                    }
                }
                
                // Notifications
                Section("Notifications") {
                    Toggle("Enable Notifications", isOn: $notificationsEnabled)
                    NavigationLink(destination: SoundSettingsView()) {
                        Label("Sound", systemImage: "speaker.fill")
                    }
                }
                
                // Appearance
                Section("Appearance") {
                    Toggle("Dark Mode", isOn: $darkModeEnabled)
                }
                
                // Storage
                Section("Storage & Data") {
                    NavigationLink(destination: StorageUsageView()) {
                        Label("Storage Usage", systemImage: "internaldrive")
                    }
                    Button(action: { /* Clear cache */ }) {
                        Label("Clear Cache", systemImage: "trash.fill")
                            .foregroundColor(.red)
                    }
                }
                
                // Help & Support
                Section("Help & Support") {
                    NavigationLink(destination: ContactSupportView()) {
                        Label("Contact Support", systemImage: "envelope.fill")
                    }
                    Button(action: { showPrivacy = true }) {
                        Label("Privacy Policy", systemImage: "doc.text")
                    }
                    Button(action: { showAbout = true }) {
                        Label("About", systemImage: "info.circle.fill")
                    }
                }
                
                // Sign Out
                Section {
                    Button(action: { /* Sign out */ }) {
                        Label("Sign Out", systemImage: "arrowtrightcircle.fill")
                            .foregroundColor(.red)
                    }
                }
            }
            .navigationTitle("Settings")
            .sheet(isPresented: $showAbout) {
                AboutView()
            }
        }
    }
}

struct SecuritySettingsView: View {
    @State private var activeDevices: [Device] = []
    
    var body: some View {
        List {
            Section("Active Devices") {
                ForEach(activeDevices, id: \.id) { device in
                    VStack(alignment: .leading) {
                        Text(device.name)
                            .fontWeight(.semibold)
                        Text(device.osVersion)
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text(device.lastSeen)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
            }
            
            Section("Recovery") {
                NavigationLink(destination: BackupCodesView()) {
                    Label("Backup Codes", systemImage: "key.fill")
                }
            }
            
            Section("Sessions") {
                Button(action: { /* Sign out all */ }) {
                    Label("Sign Out All Devices", systemImage: "arrowtrightcircle.fill")
                        .foregroundColor(.red)
                }
            }
        }
        .navigationTitle("Security")
        .onAppear {
            loadActiveDevices()
        }
    }
    
    private func loadActiveDevices() {
        activeDevices = [
            Device(id: "1", name: "iPhone 14", osVersion: "iOS 17.0", lastSeen: "Now"),
            Device(id: "2", name: "iPad", osVersion: "iPadOS 17.0", lastSeen: "2 hours ago")
        ]
    }
}

struct Device: Identifiable {
    let id: String
    let name: String
    let osVersion: String
    let lastSeen: String
}

struct ProfileEditView: View {
    @State private var displayName = "John Doe"
    @State private var email = "john@example.com"
    
    var body: some View {
        Form {
            Section("Personal Information") {
                TextField("Display Name", text: $displayName)
                TextField("Email", text: $email)
                    .keyboardType(.emailAddress)
                    .autocapitalization(.none)
            }
            
            Section {
                Button("Save Changes") { /* Save */ }
                    .frame(maxWidth: .infinity)
                    .disabled(displayName.isEmpty || email.isEmpty)
            }
        }
        .navigationTitle("Edit Profile")
    }
}

struct PasswordChangeView: View {
    @State private var currentPassword = ""
    @State private var newPassword = ""
    @State private var confirmPassword = ""
    @State private var showCurrentPassword = false
    @State private var showNewPassword = false
    
    var isFormValid: Bool {
        !currentPassword.isEmpty && !newPassword.isEmpty
            && newPassword == confirmPassword && newPassword.count >= 8
    }
    
    var body: some View {
        Form {
            Section("Current Password") {
                HStack {
                    if showCurrentPassword {
                        TextField("Current Password", text: $currentPassword)
                    } else {
                        SecureField("Current Password", text: $currentPassword)
                    }
                    Button(action: { showCurrentPassword.toggle() }) {
                        Image(systemName: showCurrentPassword ? "eye.slash.fill" : "eye.fill")
                    }
                }
            }
            
            Section("New Password") {
                HStack {
                    if showNewPassword {
                        TextField("New Password", text: $newPassword)
                    } else {
                        SecureField("New Password", text: $newPassword)
                    }
                    Button(action: { showNewPassword.toggle() }) {
                        Image(systemName: showNewPassword ? "eye.slash.fill" : "eye.fill")
                    }
                }
                SecureField("Confirm Password", text: $confirmPassword)
                
                if !newPassword.isEmpty && newPassword.count < 8 {
                    Text("Password must be at least 8 characters")
                        .foregroundColor(.red)
                        .font(.caption)
                }
            }
            
            Section {
                Button("Change Password") { /* Change */ }
                    .frame(maxWidth: .infinity)
                    .disabled(!isFormValid)
            }
        }
        .navigationTitle("Change Password")
    }
}

struct TwoFactorView: View {
    @State private var isTwoFactorEnabled = false
    @State private var showSetup = false
    
    var body: some View {
        List {
            Section("Two-Factor Authentication") {
                Toggle("Enable 2FA", isOn: $isTwoFactorEnabled)
                Text("Protect your account with two-factor authentication")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            if isTwoFactorEnabled {
                Section("Methods") {
                    HStack {
                        Label("Time-based Code (TOTP)", systemImage: "timer")
                        Spacer()
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(.green)
                    }
                    Label("SMS", systemImage: "message.fill")
                    Label("Email", systemImage: "envelope.fill")
                }
            }
        }
        .navigationTitle("Two-Factor Authentication")
    }
}

struct MessagePrivacyView: View {
    @State private var autoDeleteMessages = false
    @State private var deleteAfterHours = 24
    
    var body: some View {
        List {
            Section("Auto-Delete Messages") {
                Toggle("Enable Auto-Delete", isOn: $autoDeleteMessages)
                
                if autoDeleteMessages {
                    Stepper("Delete after \(deleteAfterHours) hours", value: $deleteAfterHours, in: 1...720)
                }
            }
            
            Section {
                Text("Messages will be automatically deleted from your device after the specified time.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .navigationTitle("Message Privacy")
    }
}

struct VisibilityView: View {
    @State private var showOnlineStatus = true
    @State private var showLastSeen = true
    @State private var allowCalls = true
    
    var body: some View {
        List {
            Section("Status") {
                Toggle("Show Online Status", isOn: $showOnlineStatus)
                Toggle("Show Last Seen", isOn: $showLastSeen)
            }
            
            Section("Calls") {
                Toggle("Allow Incoming Calls", isOn: $allowCalls)
            }
        }
        .navigationTitle("Visibility")
    }
}

struct SoundSettingsView: View {
    @State private var notificationSound = "Default"
    
    var body: some View {
        List {
            Section("Notification Sound") {
                Picker("Sound", selection: $notificationSound) {
                    Text("Default").tag("Default")
                    Text("Chime").tag("Chime")
                    Text("Glass").tag("Glass")
                    Text("None").tag("None")
                }
            }
        }
        .navigationTitle("Sound Settings")
    }
}

struct StorageUsageView: View {
    var body: some View {
        List {
            Section("Storage") {
                HStack {
                    Text("Used")
                    Spacer()
                    Text("2.3 GB")
                }
                HStack {
                    Text("Available")
                    Spacer()
                    Text("97.7 GB")
                }
            }
        }
        .navigationTitle("Storage Usage")
    }
}

struct ContactSupportView: View {
    var body: some View {
        List {
            Section {
                Link("Email Support", destination: URL(string: "mailto:support@nexus.app") ?? URL(fileURLWithPath: ""))
                Link("Website", destination: URL(string: "https://nexus.app/support") ?? URL(fileURLWithPath: ""))
            }
        }
        .navigationTitle("Support")
    }
}

struct BackupCodesView: View {
    @State private var backupCodes: [String] = ["1234-5678", "9012-3456", "7890-1234", "5678-9012", "3456-7890", "1234-5678", "9012-3456", "7890-1234"]
    
    var body: some View {
        List {
            Section("Backup Codes") {
                ForEach(backupCodes, id: \.self) { code in
                    Text(code)
                        .font(.monospaced(.system(size: 14, weight: .semibold, design: .monospaced)))
                }
            }
            
            Section {
                Button("Copy Codes") {
                    let codesText = backupCodes.joined(separator: "\n")
                    UIPasteboard.general.string = codesText
                }
                Button("Download") { /* Download */ }
            }
        }
        .navigationTitle("Backup Codes")
    }
}

struct AboutView: View {
    var body: some View {
        VStack(spacing: 16) {
            Text("Nexus Messenger")
                .font(.title)
                .fontWeight(.bold)
            
            Text("Version 1.0.0")
                .foregroundColor(.secondary)
            
            Text("End-to-end encrypted messaging for everyone")
                .font(.caption)
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
            
            Spacer()
            
            Link("Visit Website", destination: URL(string: "https://nexus.app") ?? URL(fileURLWithPath: ""))
                .buttonStyle(.bordered)
            
            Spacer()
        }
        .padding()
        .navigationTitle("About")
    }
}

#Preview {
    SettingsView()
}
