// iOS Chat View - SwiftUI
// nexus-ios/Sources/NexusMessenger/Views/ChatView.swift

import SwiftUI
import Foundation

struct Message: Identifiable {
    let id: String
    let senderId: String
    let senderName: String
    let content: String
    let timestamp: Date
    let isEncrypted: Bool
}

struct ChatView: View {
    @State private var messages: [Message] = []
    @State private var inputText: String = ""
    @State private var showCallMenu: Bool = false
    
    let conversationId: String
    let recipientName: String
    
    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading) {
                    Text(recipientName)
                        .font(.headline)
                    Text("Online")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                Spacer()
                
                Menu {
                    Button(action: { initiateCall() }) {
                        Label("Voice Call", systemImage: "phone.fill")
                    }
                    Button(action: { initiateVideoCall() }) {
                        Label("Video Call", systemImage: "video.fill")
                    }
                    Button(action: { /* Info */ }) {
                        Label("Conversation Info", systemImage: "info.circle")
                    }
                } label: {
                    Image(systemName: "ellipsis")
                        .imageScale(.large)
                }
            }
            .padding()
            .background(Color(.systemBackground))
            .border(.gray, width: 0.5)
            
            // Messages List
            ScrollViewReader { scrollProxy in
                ScrollView {
                    LazyVStack(spacing: 8) {
                        ForEach(messages) { message in
                            ChatMessageBubble(message: message, isOwnMessage: message.senderId == "currentUser")
                                .id(message.id)
                        }
                    }
                    .padding()
                }
                .onChange(of: messages.count) { _, _ in
                    if let lastMessage = messages.last {
                        withAnimation {
                            scrollProxy.scrollTo(lastMessage.id, anchor: .bottom)
                        }
                    }
                }
            }
            
            Divider()
            
            // Input Area
            HStack(spacing: 8) {
                Button(action: { /* Attach */ }) {
                    Image(systemName: "paperclip")
                        .font(.system(size: 18))
                        .foregroundColor(.blue)
                }
                
                TextField("Message...", text: $inputText)
                    .textFieldStyle(.roundedBorder)
                    .frame(height: 40)
                
                Button(action: sendMessage) {
                    Image(systemName: "paperplane.fill")
                        .font(.system(size: 18))
                        .foregroundColor(.blue)
                }
            }
            .padding()
            .background(Color(.systemBackground))
        }
        .navigationTitle("Chat")
        .onAppear {
            loadMessages()
        }
    }
    
    private func sendMessage() {
        guard !inputText.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        
        let newMessage = Message(
            id: UUID().uuidString,
            senderId: "currentUser",
            senderName: "You",
            content: inputText,
            timestamp: Date(),
            isEncrypted: true
        )
        messages.append(newMessage)
        inputText = ""
    }
    
    private func loadMessages() {
        // Load from API
    }
    
    private func initiateCall() {
        // Initiate voice call
    }
    
    private func initiateVideoCall() {
        // Initiate video call
    }
}

struct ChatMessageBubble: View {
    let message: Message
    let isOwnMessage: Bool
    
    var body: some View {
        HStack(alignment: .bottom, spacing: 8) {
            if isOwnMessage {
                Spacer()
            }
            
            VStack(alignment: isOwnMessage ? .trailing : .leading, spacing: 2) {
                Text(message.content)
                    .padding(12)
                    .foregroundColor(isOwnMessage ? .white : .primary)
                    .background(isOwnMessage ? Color.blue : Color(.systemGray5))
                    .cornerRadius(16)
                
                HStack(spacing: 4) {
                    Text(message.timestamp, style: .time)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                    
                    if message.isEncrypted {
                        Image(systemName: "lock.fill")
                            .font(.caption2)
                            .foregroundColor(.green)
                    }
                }
                .padding(.horizontal, 12)
            }
            
            if !isOwnMessage {
                Spacer()
            }
        }
    }
}

struct GroupChatView: View {
    @State private var messages: [Message] = []
    @State private var inputText: String = ""
    @State private var showMemberList: Bool = false
    
    let groupId: String
    let groupName: String
    let memberCount: Int
    
    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading) {
                    Text(groupName)
                        .font(.headline)
                    Text("\(memberCount) members")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                Spacer()
                
                Button(action: { showMemberList.toggle() }) {
                    Image(systemName: "person.2.fill")
                        .imageScale(.large)
                }
            }
            .padding()
            .background(Color(.systemBackground))
            .border(.gray, width: 0.5)
            
            // Messages
            ScrollViewReader { scrollProxy in
                ScrollView {
                    LazyVStack(spacing: 8) {
                        ForEach(messages) { message in
                            GroupMessageBubble(message: message)
                                .id(message.id)
                        }
                    }
                    .padding()
                }
                .onChange(of: messages.count) { _, _ in
                    if let lastMessage = messages.last {
                        withAnimation {
                            scrollProxy.scrollTo(lastMessage.id, anchor: .bottom)
                        }
                    }
                }
            }
            
            Divider()
            
            // Input
            HStack(spacing: 8) {
                TextField("Message...", text: $inputText)
                    .textFieldStyle(.roundedBorder)
                    .frame(height: 40)
                
                Button(action: sendMessage) {
                    Image(systemName: "paperplane.fill")
                        .foregroundColor(.blue)
                }
            }
            .padding()
        }
        .sheet(isPresented: $showMemberList) {
            GroupMembersView()
        }
    }
    
    private func sendMessage() {
        guard !inputText.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        
        let newMessage = Message(
            id: UUID().uuidString,
            senderId: "currentUser",
            senderName: "You",
            content: inputText,
            timestamp: Date(),
            isEncrypted: true
        )
        messages.append(newMessage)
        inputText = ""
    }
}

struct GroupMessageBubble: View {
    let message: Message
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(message.senderName)
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(.blue)
            
            HStack {
                Text(message.content)
                    .padding(12)
                    .foregroundColor(.primary)
                    .background(Color(.systemGray5))
                    .cornerRadius(12)
                
                Spacer()
            }
            
            HStack(spacing: 4) {
                Text(message.timestamp, style: .time)
                    .font(.caption2)
                    .foregroundColor(.secondary)
                
                if message.isEncrypted {
                    Image(systemName: "lock.fill")
                        .font(.caption2)
                        .foregroundColor(.green)
                }
            }
            .padding(.horizontal, 12)
        }
    }
}

struct GroupMembersView: View {
    @State private var members: [String] = ["Alice", "Bob", "Charlie"]
    
    var body: some View {
        NavigationStack {
            List(members, id: \.self) { member in
                HStack {
                    Circle()
                        .fill(Color.blue)
                        .frame(width: 40, height: 40)
                        .overlay(
                            Text(String(member.prefix(1)))
                                .foregroundColor(.white)
                                .fontWeight(.bold)
                        )
                    
                    VStack(alignment: .leading) {
                        Text(member)
                            .fontWeight(.semibold)
                        Text("Online")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    
                    Spacer()
                }
            }
            .navigationTitle("Group Members")
        }
    }
}

#Preview {
    ChatView(conversationId: "1", recipientName: "Alice")
}
