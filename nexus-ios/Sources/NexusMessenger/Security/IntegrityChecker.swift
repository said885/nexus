import Foundation
import UIKit
import Darwin

// MARK: - Threat Level

public enum ThreatLevel: Int, Comparable {
    case none = 0, low = 1, medium = 2, high = 3, critical = 4

    public static func < (lhs: ThreatLevel, rhs: ThreatLevel) -> Bool {
        lhs.rawValue < rhs.rawValue
    }
}

// MARK: - IntegrityChecker

public final class IntegrityChecker {

    private init() {}

    // -------------------------------------------------------------------------
    // MARK: Full Check
    // -------------------------------------------------------------------------

    public static func performFullCheck() -> ThreatLevel {
        let checks: [ThreatLevel] = [
            checkJailbreakFiles(),
            checkSandboxEscape(),
            checkDynamicLibraries(),
            checkSymlinks(),
            checkEnvironmentVariables(),
            checkOpenPorts(),
            checkDebugger(),
            checkAppIntegrity(),
        ]
        return checks.max() ?? .none
    }

    // -------------------------------------------------------------------------
    // MARK: Individual Checks
    // -------------------------------------------------------------------------

    private static func checkJailbreakFiles() -> ThreatLevel {
        let jbPaths = [
            "/Applications/Cydia.app",
            "/Applications/blackra1n.app",
            "/Applications/FakeCarrier.app",
            "/Applications/Icy.app",
            "/Applications/IntelliScreen.app",
            "/Applications/MxTube.app",
            "/Applications/RockApp.app",
            "/Applications/SBSettings.app",
            "/Applications/WinterBoard.app",
            "/Library/MobileSubstrate/MobileSubstrate.dylib",
            "/Library/MobileSubstrate/DynamicLibraries/LiveClock.plist",
            "/Library/MobileSubstrate/DynamicLibraries/Veency.plist",
            "/private/var/lib/apt/",
            "/private/var/lib/cydia",
            "/private/var/mobile/Library/SBSettings/Themes",
            "/private/var/stash",
            "/private/var/tmp/cydia.log",
            "/bin/bash",
            "/bin/sh",
            "/usr/sbin/sshd",
            "/usr/bin/sshd",
            "/usr/libexec/ssh-keysign",
            "/usr/bin/ssh",
            "/usr/libexec/sftp-server",
            "/etc/apt",
            "/etc/ssh/sshd_config",
            "/var/jb",
            "/.bootstrapped_electra",
            "/usr/lib/libjailbreak.dylib",
            "/jb/lzma",
            "/var/binpack",
            "/Library/MobileSubstrate",
            "/usr/share/jailbreak_dylibs",
        ]
        for path in jbPaths {
            if FileManager.default.fileExists(atPath: path) {
                return .high
            }
        }
        // Try opening paths that should be inaccessible
        let restrictedPaths = ["/bin/bash", "/bin/sh", "/usr/sbin/sshd"]
        for path in restrictedPaths {
            if let _ = try? String(contentsOfFile: path, encoding: .utf8) {
                return .high
            }
        }
        return .none
    }

    private static func checkSandboxEscape() -> ThreatLevel {
        let testPath = "/private/nexus_sandbox_test_\(UUID().uuidString)"
        do {
            try "nexus_test".write(toFile: testPath, atomically: true, encoding: .utf8)
            // If the write succeeds, the sandbox has been escaped
            try? FileManager.default.removeItem(atPath: testPath)
            return .critical
        } catch {
            return .none // Expected — sandbox is intact
        }
    }

    private static func checkDynamicLibraries() -> ThreatLevel {
        // Check for Substrate, Frida, Cycript injection markers
        let suspiciousLibraryPatterns = [
            "MobileSubstrate",
            "CydiaSubstrate",
            "FridaGadget",
            "frida",
            "cynject",
            "cycript",
            "SubstrateBootstrap",
            "libhooker",
        ]

        let imageCount = _dyld_image_count()
        for i in 0..<imageCount {
            guard let name = _dyld_get_image_name(i) else { continue }
            let imageName = String(cString: name).lowercased()
            for pattern in suspiciousLibraryPatterns {
                if imageName.contains(pattern.lowercased()) {
                    return .high
                }
            }
        }

        // Check for Frida gadget in memory (common ports)
        return .none
    }

    private static func checkDebugger() -> ThreatLevel {
        var info = kinfo_proc()
        var mib: [Int32] = [CTL_KERN, KERN_PROC, KERN_PROC_PID, getpid()]
        var size = MemoryLayout<kinfo_proc>.stride
        let result = sysctl(&mib, UInt32(mib.count), &info, &size, nil, 0)
        guard result == 0 else { return .none }
        let isBeingTraced = (info.kp_proc.p_flag & P_TRACED) != 0
        return isBeingTraced ? .critical : .none
    }

    private static func checkAppIntegrity() -> ThreatLevel {
        // Verify that the main bundle has a valid code signature.
        // SecStaticCodeCheckValidityWithErrors requires the Security framework at macOS,
        // but on iOS we use a lighter heuristic: check that the embedded mobile provision
        // or the Info.plist has not been tampered with.
        guard let bundleURL = Bundle.main.bundleURL as CFURL? else { return .medium }

        var staticCode: SecStaticCode?
        let createStatus = SecStaticCodeCreateWithPath(bundleURL, [], &staticCode)
        guard createStatus == errSecSuccess, let code = staticCode else {
            return .medium
        }

        var requirement: SecRequirement?
        // Use the hard-coded designated requirement for NEXUS
        let reqString = "anchor apple generic" as CFString
        let reqStatus = SecRequirementCreateWithString(reqString, [], &requirement)
        guard reqStatus == errSecSuccess, let req = requirement else {
            // If we can't create requirement, treat as low risk
            return .low
        }

        var cfError: Unmanaged<CFError>?
        let checkStatus = SecStaticCodeCheckValidityWithErrors(
            code,
            SecCSFlags(rawValue: 0),
            req,
            &cfError
        )
        if let error = cfError?.takeRetainedValue() {
            _ = error // log in production
        }
        return checkStatus == errSecSuccess ? .none : .high
    }

    private static func checkOpenPorts() -> ThreatLevel {
        // Check if Frida default port (27042) or common debug ports are open
        let suspiciousPorts: [Int32] = [27042, 27043, 4444, 1337]
        for port in suspiciousPorts {
            if isPortOpen(port: port) {
                return .high
            }
        }
        return .none
    }

    private static func isPortOpen(port: Int32) -> Bool {
        let sockfd = socket(AF_INET, SOCK_STREAM, 0)
        guard sockfd >= 0 else { return false }
        defer { close(sockfd) }

        var addr = sockaddr_in()
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_port = UInt16(port).bigEndian
        addr.sin_addr.s_addr = inet_addr("127.0.0.1")

        let result = withUnsafePointer(to: &addr) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockaddrPtr in
                connect(sockfd, sockaddrPtr, socklen_t(MemoryLayout<sockaddr_in>.size))
            }
        }
        return result == 0
    }

    private static func checkEnvironmentVariables() -> ThreatLevel {
        let suspiciousVars = [
            "DYLD_INSERT_LIBRARIES",
            "DYLD_FORCE_FLAT_NAMESPACE",
            "DYLD_LIBRARY_PATH",
            "_MSSafeMode",
        ]
        for varName in suspiciousVars {
            if let value = ProcessInfo.processInfo.environment[varName], !value.isEmpty {
                return .high
            }
            if getenv(varName) != nil {
                return .high
            }
        }
        return .none
    }

    private static func checkSymlinks() -> ThreatLevel {
        // On a stock iOS device, /Applications should NOT be a symlink.
        // Jailbreaks commonly replace it with a symlink to /var/stash/Applications.
        let applicationsPath = "/Applications"
        let attributes = try? FileManager.default.attributesOfItem(atPath: applicationsPath)
        if let fileType = attributes?[.type] as? FileAttributeType,
           fileType == .typeSymbolicLink {
            return .high
        }

        // Also check /Library
        let libraryPath = "/Library"
        let libAttributes = try? FileManager.default.attributesOfItem(atPath: libraryPath)
        if let fileType = libAttributes?[.type] as? FileAttributeType,
           fileType == .typeSymbolicLink {
            return .medium
        }

        return .none
    }

    // -------------------------------------------------------------------------
    // MARK: Threat Response
    // -------------------------------------------------------------------------

    public static func handleThreat(_ level: ThreatLevel) {
        switch level {
        case .none:
            break
        case .low:
            NotificationCenter.default.post(name: .nexusSecurityWarning, object: level)
        case .medium:
            NotificationCenter.default.post(name: .nexusRestrictedMode, object: level)
        case .high:
            NotificationCenter.default.post(name: .nexusPanicMode, object: level)
        case .critical:
            performEmergencyWipe()
        }
    }

    // -------------------------------------------------------------------------
    // MARK: Emergency Wipe
    // -------------------------------------------------------------------------

    public static func performEmergencyWipe() {
        // 1. Delete all Keychain items belonging to this app
        let secClasses: [CFString] = [
            kSecClassGenericPassword,
            kSecClassInternetPassword,
            kSecClassCertificate,
            kSecClassKey,
            kSecClassIdentity,
        ]
        for secClass in secClasses {
            let query: [String: Any] = [kSecClass as String: secClass]
            SecItemDelete(query as CFDictionary)
        }

        // 2. Delete all app container files
        let fileManager = FileManager.default
        let dirs: [URL] = [
            fileManager.urls(for: .documentDirectory, in: .userDomainMask).first,
            fileManager.urls(for: .libraryDirectory,  in: .userDomainMask).first,
            fileManager.urls(for: .cachesDirectory,   in: .userDomainMask).first,
        ].compactMap { $0 }

        for dir in dirs {
            if let contents = try? fileManager.contentsOfDirectory(
                at: dir, includingPropertiesForKeys: nil
            ) {
                for url in contents {
                    try? fileManager.removeItem(at: url)
                }
            }
        }

        // 3. Clear UserDefaults
        if let domain = Bundle.main.bundleIdentifier {
            UserDefaults.standard.removePersistentDomain(forName: domain)
        }
        UserDefaults.standard.synchronize()

        // 4. Notify the UI to close and post the emergency wipe notification
        DispatchQueue.main.async {
            NotificationCenter.default.post(name: .nexusEmergencyWipe, object: nil)
        }
    }
}

// MARK: - Notification Names

public extension Notification.Name {
    static let nexusSecurityWarning = Notification.Name("com.nexus.securityWarning")
    static let nexusRestrictedMode  = Notification.Name("com.nexus.restrictedMode")
    static let nexusPanicMode       = Notification.Name("com.nexus.panicMode")
    static let nexusEmergencyWipe   = Notification.Name("com.nexus.emergencyWipe")
}
