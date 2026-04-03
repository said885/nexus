// Android Conversation List Screen - Jetpack Compose
// nexus-android/src/main/java/com/nexus/ui/screen/ConversationListScreen.kt

package com.nexus.ui.screen

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nexus.data.model.Conversation

data class ConversationItem(
    val id: String,
    val name: String,
    val lastMessage: String,
    val timestamp: String,
    val unreadCount: Int,
    val isGroup: Boolean,
    val avatarUrl: String?
)

@Composable
fun ConversationListScreen(
    conversations: List<ConversationItem>,
    onConversationClick: (String) -> Unit,
    onNewConversation: () -> Unit,
    onSearchClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
    ) {
        // Top App Bar
        TopAppBar(
            title = { Text("Nexus Messenger") },
            actions = {
                IconButton(onClick = onSearchClick) {
                    Icon(Icons.Default.Search, contentDescription = "Search")
                }
                IconButton(onClick = { /* Settings */ }) {
                    Icon(Icons.Default.Settings, contentDescription = "Settings")
                }
            },
            colors = TopAppBarDefaults.topAppBarColors(
                containerColor = MaterialTheme.colorScheme.primary
            )
        )
        
        if (conversations.isEmpty()) {
            // Empty state
            Box(
                modifier = Modifier
                    .fillMaxSize(),
                contentAlignment = Alignment.Center
            ) {
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    modifier = Modifier.padding(16.dp)
                ) {
                    Icon(
                        Icons.Default.Chat,
                        contentDescription = "No conversations",
                        modifier = Modifier.size(64.dp),
                        tint = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(modifier = Modifier.height(16.dp))
                    Text(
                        "No conversations yet",
                        fontSize = 16.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Button(onClick = onNewConversation) {
                        Text("Start a conversation")
                    }
                }
            }
        } else {
            // Conversations List
            LazyColumn(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()
            ) {
                items(conversations) { conversation ->
                    ConversationListItem(
                        conversation = conversation,
                        onClick = { onConversationClick(conversation.id) }
                    )
                }
            }
        }
        
        // Floating Action Button
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            contentAlignment = Alignment.BottomEnd
        ) {
            FloatingActionButton(
                onClick = onNewConversation,
                containerColor = MaterialTheme.colorScheme.primary
            ) {
                Icon(Icons.Default.Add, contentDescription = "New conversation")
            }
        }
    }
}

@Composable
fun ConversationListItem(
    conversation: ConversationItem,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Surface(
        modifier = modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        color = MaterialTheme.colorScheme.surface
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Avatar
            Surface(
                modifier = Modifier
                    .size(56.dp)
                    .clip(CircleShape),
                color = MaterialTheme.colorScheme.primaryContainer
            ) {
                Box(
                    contentAlignment = Alignment.Center,
                    modifier = Modifier.fillMaxSize()
                ) {
                    Text(
                        text = conversation.name.first().toString(),
                        fontSize = 24.sp,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onPrimaryContainer
                    )
                }
            }
            
            Spacer(modifier = Modifier.width(12.dp))
            
            // Content
            Column(
                modifier = Modifier
                    .weight(1f)
                    .align(Alignment.CenterVertically)
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = conversation.name,
                        fontSize = 16.sp,
                        fontWeight = FontWeight.SemiBold,
                        color = MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.weight(1f)
                    )
                    Text(
                        text = conversation.timestamp,
                        fontSize = 12.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
                
                Spacer(modifier = Modifier.height(4.dp))
                
                Row(
                    modifier = Modifier
                        .fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = conversation.lastMessage,
                        fontSize = 13.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                        modifier = Modifier.weight(1f)
                    )
                    
                    if (conversation.unreadCount > 0) {
                        Badge(
                            modifier = Modifier
                                .clip(CircleShape)
                                .background(MaterialTheme.colorScheme.error)
                                .padding(4.dp)
                        ) {
                            Text(
                                text = conversation.unreadCount.toString(),
                                fontSize = 10.sp,
                                color = MaterialTheme.colorScheme.onError
                            )
                        }
                    }
                }
            }
        }
    }
    Divider()
}

@Composable
fun SearchConversationScreen(
    onBackClick: () -> Unit,
    onConversationClick: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    var searchQuery by remember { mutableStateOf("") }
    var searchResults by remember { mutableStateOf<List<ConversationItem>>(emptyList()) }
    
    Column(
        modifier = modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
    ) {
        // Search Bar
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            IconButton(onClick = onBackClick) {
                Icon(Icons.Default.ArrowBack, contentDescription = "Back")
            }
            
            OutlinedTextField(
                value = searchQuery,
                onValueChange = {
                    searchQuery = it
                    // Perform search
                },
                modifier = Modifier
                    .weight(1f)
                    .height(40.dp),
                placeholder = { Text("Search conversations...") },
                singleLine = true,
                shape = RoundedCornerShape(24.dp),
                leadingIcon = {
                    Icon(Icons.Default.Search, contentDescription = "Search")
                }
            )
        }
        
        // Results
        LazyColumn(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
        ) {
            items(searchResults) { result ->
                ConversationListItem(
                    conversation = result,
                    onClick = { onConversationClick(result.id) }
                )
            }
        }
    }
}
