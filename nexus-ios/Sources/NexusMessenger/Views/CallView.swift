// iOS Call View - SwiftUI
// nexus-ios/Sources/NexusMessenger/Views/CallView.swift

import SwiftUI
import AVFoundation

enum CallType {
    case voice
    case video
}

enum CallState {
    case dialing
    case connecting
    case active
    case ended
}

struct CallView: View {
    @State private var callState = CallState.connecting
    @State private var isMuted = false
    @State private var isCameraOn = true
    @State private var callDuration = 0
    @State private var timer: Timer?
    
    let recipientName: String
    let callType: CallType
    let onEndCall: () -> Void
    
    var body: some View {
        ZStack {
            // Background
            Color.black
                .ignoresSafeArea()
            
            // Call Content
            VStack(spacing: 0) {
                // Top Info
                VStack(spacing: 8) {
                    Text(recipientName)
                        .font(.title2)
                        .fontWeight(.semibold)
                        .foregroundColor(.white)
                    
                    Text(formatDuration(callDuration))
                        .font(.body)
                        .foregroundColor(.gray)
                    
                    Text(callState == .connecting ? "Connecting..." : "Connected")
                        .font(.caption)
                        .foregroundColor(.gray)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 24)
                
                Spacer()
                
                // Video Area (if video call)
                if callType == .video {
                    ZStack(alignment: .topTrailing) {
                        // Remote video placeholder
                        RoundedRectangle(cornerRadius: 12)
                            .fill(Color.gray.opacity(0.2))
                        
                        // Local video thumbnail
                        RoundedRectangle(cornerRadius: 8)
                            .fill(Color.gray.opacity(0.4))
                            .frame(width: 100, height: 150)
                            .overlay(
                                Text("You")
                                    .foregroundColor(.white)
                                    .fontWeight(.semibold)
                            )
                            .padding(12)
                    }
                    .frame(height: 400)
                    .padding(12)
                }
                
                Spacer()
                
                // Control Buttons
                HStack(spacing: 24) {
                    // Mute Button
                    CallControlButton(
                        icon: isMuted ? "mic.slash.fill" : "mic.fill",
                        backgroundColor: isMuted ? .red : .gray.opacity(0.3),
                        action: { isMuted.toggle() }
                    )
                    
                    // Camera Button (video calls only)
                    if callType == .video {
                        CallControlButton(
                            icon: isCameraOn ? "video.fill" : "video.slash.fill",
                            backgroundColor: .gray.opacity(0.3),
                            action: { isCameraOn.toggle() }
                        )
                    }
                    
                    Spacer()
                    
                    // End Call Button
                    CallControlButton(
                        icon: "phone.fill",
                        backgroundColor: .red,
                        action: {
                            callState = .ended
                            timer?.invalidate()
                            onEndCall()
                        }
                    )
                }
                .padding(24)
                .frame(maxWidth: .infinity)
            }
        }
        .onAppear {
            startCallTimer()
        }
        .onDisappear {
            timer?.invalidate()
        }
    }
    
    private func startCallTimer() {
        timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { _ in
            callDuration += 1
        }
    }
    
    private func formatDuration(_ seconds: Int) -> String {
        let minutes = seconds / 60
        let secs = seconds % 60
        return String(format: "%02d:%02d", minutes, secs)
    }
}

struct CallControlButton: View {
    let icon: String
    let backgroundColor: Color
    let action: () -> Void
    
    var body: some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.system(size: 20))
                .foregroundColor(.white)
                .frame(width: 56, height: 56)
                .background(backgroundColor)
                .clipShape(Circle())
        }
    }
}

struct IncomingCallView: View {
    let callerName: String
    let onAccept: () -> Void
    let onReject: () -> Void
    
    var body: some View {
        ZStack {
            // Gradient Background
            LinearGradient(
                gradient: Gradient(colors: [
                    Color(red: 0.4, green: 0.5, blue: 0.92),
                    Color(red: 0.47, green: 0.3, blue: 0.71)
                ]),
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .ignoresSafeArea()
            
            VStack(spacing: 32) {
                Spacer()
                
                // Avatar
                ZStack {
                    Circle()
                        .fill(Color.white.opacity(0.2))
                        .frame(width: 120, height: 120)
                    
                    Text(String(callerName.prefix(1)))
                        .font(.system(size: 56, weight: .semibold))
                        .foregroundColor(.white)
                }
                
                VStack(spacing: 12) {
                    Text(callerName)
                        .font(.title)
                        .fontWeight(.semibold)
                        .foregroundColor(.white)
                    
                    Text("Incoming call...")
                        .font(.body)
                        .foregroundColor(.white.opacity(0.8))
                }
                
                Spacer()
                
                // Accept / Reject Buttons
                HStack(spacing: 32) {
                    // Reject
                    Button(action: onReject) {
                        Image(systemName: "phone.fill")
                            .font(.system(size: 24))
                            .foregroundColor(.white)
                            .frame(width: 64, height: 64)
                            .background(Color.red)
                            .clipShape(Circle())
                    }
                    
                    Spacer()
                    
                    // Accept
                    Button(action: onAccept) {
                        Image(systemName: "phone.fill")
                            .font(.system(size: 24))
                            .foregroundColor(.white)
                            .frame(width: 64, height: 64)
                            .background(Color.green)
                            .clipShape(Circle())
                    }
                }
                .padding(.horizontal, 32)
                .padding(.bottom, 32)
            }
        }
    }
}

#Preview {
    CallView(recipientName: "Alice", callType: .video) { }
}
