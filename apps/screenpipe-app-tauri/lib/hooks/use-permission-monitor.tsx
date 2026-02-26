"use client";

import { useEffect, useRef } from "react";
import { usePathname } from "next/navigation";
import { listen } from "@tauri-apps/api/event";
import { commands } from "@/lib/utils/tauri";
import posthog from "posthog-js";

interface PermissionLostPayload {
  screen_recording: boolean;
  microphone: boolean;
  accessibility: boolean;
  arc_automation?: boolean;
}

/**
 * Hook that listens for permission-lost events from the Rust backend
 * and automatically shows the permission recovery window
 */
export function usePermissionMonitor() {
  const hasShownRef = useRef(false);
  const pathname = usePathname();

  useEffect(() => {
    // Only run on client side
    if (typeof window === "undefined") return;

    // Only run permission monitor on main app pages, not overlays or onboarding
    // This prevents duplicate PostHog events since each Tauri window runs its own instance
    const skipPaths = ["/shortcut-reminder", "/onboarding", "/permission-recovery"];
    if (skipPaths.some((p) => pathname?.startsWith(p))) return;

    const unlisten = listen<PermissionLostPayload>("permission-lost", async (event) => {
      const { screen_recording, microphone, accessibility, arc_automation } = event.payload;

      // Don't show multiple times in quick succession
      if (hasShownRef.current) return;

      console.log("Permission lost event received:", { screen_recording, microphone, accessibility, arc_automation });

      // Double-check core permissions before showing modal to avoid false positives
      // The backend already requires consecutive failures, but let's verify once more
      // Note: arc_automation is checked by the backend via osascript, no need to re-verify here
      try {
        const currentPerms = await commands.doPermissionsCheck(false);
        const screenOk = currentPerms.screenRecording === "granted" || currentPerms.screenRecording === "notNeeded";
        const micOk = currentPerms.microphone === "granted" || currentPerms.microphone === "notNeeded";
        const accessibilityOk = currentPerms.accessibility === "granted" || currentPerms.accessibility === "notNeeded";

        // Show modal if ANY permission is lost (screen, mic, accessibility, OR arc automation)
        if (screenOk && micOk && accessibilityOk && !arc_automation) {
          console.log("Permission check passed on frontend verification, skipping modal");
          return;
        }

        console.log("Permission loss confirmed:", { screenOk, micOk, accessibilityOk, arc_automation });
      } catch (error) {
        console.error("Failed to verify permissions:", error);
        // Continue to show modal if we can't verify
      }

      hasShownRef.current = true;

      // Track the event
      posthog.capture("permission_lost", {
        screen_recording_lost: screen_recording,
        microphone_lost: microphone,
        accessibility_lost: accessibility,
        arc_automation_lost: arc_automation,
      });

      // Show the permission recovery window
      try {
        await commands.showWindow("PermissionRecovery");
      } catch (error) {
        console.error("Failed to show permission recovery window:", error);
      }

      // Reset after a delay to allow showing again later
      // Backend has a 5-minute cooldown on re-emission, so this just guards
      // against any edge cases where multiple events arrive close together
      setTimeout(() => {
        hasShownRef.current = false;
      }, 300000); // 5 minute cooldown (matches backend)
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [pathname]);
}

/**
 * Provider component that sets up the permission monitor
 */
export function PermissionMonitorProvider({ children }: { children: React.ReactNode }) {
  usePermissionMonitor();
  return <>{children}</>;
}
