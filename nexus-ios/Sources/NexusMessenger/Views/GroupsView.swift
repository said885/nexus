import SwiftUI

// MARK: - Models

struct GroupInfo: Identifiable, Codable {
    let id: String
    var name: String
    var description: String?
    let owner: String
    var members: [GroupMember]
    let createdAt: Date

    var memberCount: Int { members.count }
    var isOwner: Bool { false } // Would compare with current user
}

struct GroupMember: Identifiable, Codable {
    let id: String
    var name: String
    var isAdmin: Bool
    var isOnline: Bool
}

// MARK: - Groups View

struct GroupsView: View {
    @StateObject private var viewModel = GroupsViewModel()
    @State private var showCreateGroup = false
    @State private var selectedGroup: GroupInfo?

    var body: some View {
        NavigationView {
            Group {
                if viewModel.groups.isEmpty {
                    emptyStateView
                } else {
                    groupListView
                }
            }
            .navigationTitle("Groups")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { showCreateGroup = true }) {
                        Image(systemName: "plus")
                    }
                }
            }
            .sheet(isPresented: $showCreateGroup) {
                CreateGroupView(onCreate: { name, description in
                    viewModel.createGroup(name: name, description: description)
                    showCreateGroup = false
                })
            }
            .sheet(item: $selectedGroup) { group in
                GroupDetailView(group: group, viewModel: viewModel)
            }
        }
    }

    private var emptyStateView: some View {
        VStack(spacing: 20) {
            Image(systemName: "person.3.fill")
                .font(.system(size: 60))
                .foregroundColor(.secondary)

            Text("No Groups Yet")
                .font(.title2)
                .fontWeight(.semibold)

            Text("Create a group to start chatting with multiple people securely")
                .font(.body)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 40)

            Button(action: { showCreateGroup = true }) {
                Label("Create Group", systemImage: "plus.circle.fill")
                    .font(.headline)
                    .padding()
                    .background(Color.accentColor)
                    .foregroundColor(.white)
                    .cornerRadius(12)
            }
        }
    }

    private var groupListView: some View {
        List {
            ForEach(viewModel.groups) { group in
                GroupRow(group: group)
                    .onTapGesture {
                        selectedGroup = group
                    }
            }
            .onDelete(perform: viewModel.deleteGroup)
        }
        .listStyle(InsetGroupedListStyle())
        .refreshable {
            viewModel.loadGroups()
        }
    }
}

// MARK: - Group Row

struct GroupRow: View {
    let group: GroupInfo

    var body: some View {
        HStack(spacing: 12) {
            // Group avatar
            ZStack {
                Circle()
                    .fill(Color.accentColor.gradient)
                    .frame(width: 50, height: 50)

                Image(systemName: "person.3.fill")
                    .font(.title3)
                    .foregroundColor(.white)
            }

            VStack(alignment: .leading, spacing: 4) {
                Text(group.name)
                    .font(.headline)

                HStack {
                    Text("\(group.memberCount) members")
                        .font(.caption)
                        .foregroundColor(.secondary)

                    if let description = group.description {
                        Text("•")
                            .foregroundColor(.secondary)
                        Text(description)
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                    }
                }
            }

            Spacer()

            Image(systemName: "chevron.right")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Group Detail View

struct GroupDetailView: View {
    let group: GroupInfo
    @ObservedObject var viewModel: GroupsViewModel
    @Environment(\.dismiss) var dismiss
    @State private var showAddMember = false
    @State private var showLeaveConfirmation = false
    @State private var newMemberHash = ""

    var body: some View {
        NavigationView {
            List {
                // Group info section
                Section {
                    HStack {
                        Spacer()
                        VStack(spacing: 12) {
                            ZStack {
                                Circle()
                                    .fill(Color.accentColor.gradient)
                                    .frame(width: 80, height: 80)

                                Image(systemName: "person.3.fill")
                                    .font(.title)
                                    .foregroundColor(.white)
                            }

                            Text(group.name)
                                .font(.title2)
                                .fontWeight(.bold)

                            if let description = group.description {
                                Text(description)
                                    .font(.body)
                                    .foregroundColor(.secondary)
                                    .multilineTextAlignment(.center)
                            }
                        }
                        Spacer()
                    }
                    .padding(.vertical, 8)
                }

                // Members section
                Section(header: Text("Members (\(group.members.count))")) {
                    ForEach(group.members) { member in
                        MemberRow(member: member)
                    }
                }

                // Actions section
                Section {
                    Button(action: { showAddMember = true }) {
                        Label("Add Member", systemImage: "person.badge.plus")
                    }

                    Button(action: { showLeaveConfirmation = true }) {
                        Label("Leave Group", systemImage: "rectangle.portrait.and.arrow.right")
                            .foregroundColor(.red)
                    }
                }
            }
            .listStyle(InsetGroupedListStyle())
            .navigationTitle("Group Info")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") { dismiss() }
                }
            }
            .alert("Add Member", isPresented: $showAddMember) {
                TextField("Member Identity Hash", text: $newMemberHash)
                Button("Cancel", role: .cancel) { newMemberHash = "" }
                Button("Add") {
                    if newMemberHash.count == 64 {
                        viewModel.addMember(groupId: group.id, memberHash: newMemberHash)
                        newMemberHash = ""
                    }
                }
            } message: {
                Text("Enter the 64-character identity hash of the member to add")
            }
            .confirmationDialog("Leave Group", isPresented: $showLeaveConfirmation) {
                Button("Leave Group", role: .destructive) {
                    viewModel.leaveGroup(groupId: group.id)
                    dismiss()
                }
            } message: {
                Text("Are you sure you want to leave this group?")
            }
        }
    }
}

// MARK: - Member Row

struct MemberRow: View {
    let member: GroupMember

    var body: some View {
        HStack(spacing: 12) {
            ZStack {
                Circle()
                    .fill(Color.blue.gradient)
                    .frame(width: 40, height: 40)

                Text(String(member.name.prefix(1)).uppercased())
                    .font(.headline)
                    .foregroundColor(.white)

                if member.isOnline {
                    Circle()
                        .fill(.green)
                        .frame(width: 12, height: 12)
                        .overlay(
                            Circle()
                                .stroke(Color(.systemBackground), lineWidth: 2)
                        )
                        .offset(x: 14, y: 14)
                }
            }

            VStack(alignment: .leading, spacing: 2) {
                Text(member.name)
                    .font(.body)

                Text(member.id.prefix(16) + "...")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Spacer()

            if member.isAdmin {
                Text("Admin")
                    .font(.caption)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.accentColor.opacity(0.2))
                    .foregroundColor(.accentColor)
                    .cornerRadius(4)
            }
        }
    }
}

// MARK: - Create Group View

struct CreateGroupView: View {
    let onCreate: (String, String?) -> Void
    @Environment(\.dismiss) var dismiss
    @State private var name = ""
    @State private var description = ""

    var body: some View {
        NavigationView {
            Form {
                Section {
                    HStack {
                        Spacer()
                        ZStack {
                            Circle()
                                .fill(Color.accentColor.gradient)
                                .frame(width: 80, height: 80)

                            Image(systemName: "person.3.fill")
                                .font(.title)
                                .foregroundColor(.white)
                        }
                        Spacer()
                    }
                    .listRowBackground(Color.clear)
                }

                Section(header: Text("Group Info")) {
                    TextField("Group Name", text: $name)
                    TextField("Description (optional)", text: $description, axis: .vertical)
                        .lineLimit(3)
                }

                Section {
                    Button(action: {
                        onCreate(name, description.isEmpty ? nil : description)
                    }) {
                        HStack {
                            Spacer()
                            Label("Create Group", systemImage: "plus.circle.fill")
                                .font(.headline)
                            Spacer()
                        }
                    }
                    .disabled(name.isEmpty)
                }
            }
            .navigationTitle("New Group")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                }
            }
        }
    }
}

// MARK: - View Model

class GroupsViewModel: ObservableObject {
    @Published var groups: [GroupInfo] = []
    @Published var isLoading = false

    init() {
        loadGroups()
    }

    func loadGroups() {
        isLoading = true
        // In production, this would fetch from the server
        // For now, use sample data
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            self.groups = [
                GroupInfo(
                    id: "group-1",
                    name: "Development Team",
                    description: "Main development discussion",
                    owner: "alice-hash",
                    members: [
                        GroupMember(id: "alice-hash", name: "Alice", isAdmin: true, isOnline: true),
                        GroupMember(id: "bob-hash", name: "Bob", isAdmin: false, isOnline: true),
                        GroupMember(id: "charlie-hash", name: "Charlie", isAdmin: false, isOnline: false),
                    ],
                    createdAt: Date()
                ),
                GroupInfo(
                    id: "group-2",
                    name: "Security Team",
                    description: "Security discussions",
                    owner: "bob-hash",
                    members: [
                        GroupMember(id: "bob-hash", name: "Bob", isAdmin: true, isOnline: true),
                        GroupMember(id: "dave-hash", name: "Dave", isAdmin: false, isOnline: false),
                    ],
                    createdAt: Date()
                ),
            ]
            self.isLoading = false
        }
    }

    func createGroup(name: String, description: String?) {
        let newGroup = GroupInfo(
            id: UUID().uuidString,
            name: name,
            description: description,
            owner: "current-user",
            members: [
                GroupMember(id: "current-user", name: "You", isAdmin: true, isOnline: true)
            ],
            createdAt: Date()
        )
        groups.append(newGroup)
    }

    func deleteGroup(at offsets: IndexSet) {
        groups.remove(atOffsets: offsets)
    }

    func addMember(groupId: String, memberHash: String) {
        guard let index = groups.firstIndex(where: { $0.id == groupId }) else { return }
        let newMember = GroupMember(
            id: memberHash,
            name: String(memberHash.prefix(8)),
            isAdmin: false,
            isOnline: false
        )
        groups[index].members.append(newMember)
    }

    func leaveGroup(groupId: String) {
        groups.removeAll { $0.id == groupId }
    }
}
