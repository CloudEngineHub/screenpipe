"use client";

import React, { useEffect, useState, useMemo, Suspense } from "react";
import { useSettings } from "@/lib/hooks/use-settings";
import { usePostHog } from "posthog-js/react";
import {
  Brain,
  Video,
  Keyboard,
  User,
  Settings as SettingsIcon,
  HardDrive,
  Plug,
  MessageSquare,
  Cloud,
  Workflow,
  Users,
  Home,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { AccountSection } from "@/components/settings/account-section";
import ShortcutSection from "@/components/settings/shortcut-section";
import { AIPresets } from "@/components/settings/ai-presets";
import { RecordingSettings } from "@/components/settings/recording-settings";
import GeneralSettings from "@/components/settings/general-settings";
import { DiskUsageSection } from "@/components/settings/disk-usage-section";
import { ConnectionsSection } from "@/components/settings/connections-section";
import { FeedbackSection } from "@/components/settings/feedback-section";
import { PipesSection } from "@/components/settings/pipes-section";
import { SyncSettings } from "@/components/settings/sync-settings";
import { TeamSection } from "@/components/settings/team-section";
import { StandaloneChat } from "@/components/standalone-chat";
import { useRouter } from "next/navigation";
import { useQueryState } from "nuqs";
import { listen } from "@tauri-apps/api/event";

type SettingsSection =
  | "home"
  | "general"
  | "ai"
  | "shortcuts"
  | "recording"
  | "account"
  | "disk-usage"
  | "connections"
  | "cloud-sync"
  | "pipes"
  | "team"
  | "feedback";

function SettingsPageContent() {
  const router = useRouter();
  const [activeSection, setActiveSection] = useQueryState("section", {
    defaultValue: "home" as SettingsSection,
    parse: (value) => {
      if (["home", "general", "ai", "shortcuts", "recording", "account", "disk-usage", "connections", "cloud-sync", "pipes", "team", "feedback"].includes(value)) {
        return value as SettingsSection;
      }
      return "home" as SettingsSection;
    },
    serialize: (value) => value,
  });
  
  const { settings } = useSettings();
  const posthog = usePostHog();
  const showCloudSync = useMemo(() => posthog?.isFeatureEnabled("cloud-sync") ?? false, [posthog]);

  const renderSection = () => {
    switch (activeSection) {
      case "home":
        return <StandaloneChat className="!h-full" />;
      case "general":
        return <GeneralSettings />;
      case "ai":
        return <AIPresets />;
      case "account":
        return <AccountSection />;
      case "recording":
        return <RecordingSettings />;
      case "shortcuts":
        return <ShortcutSection />;
      case "disk-usage":
        return <DiskUsageSection />;
      case "connections":
        return <ConnectionsSection />;
      case "pipes":
        return <PipesSection />;
      case "team":
        return <TeamSection />;
      case "cloud-sync":
        return showCloudSync ? <SyncSettings /> : <GeneralSettings />;
      case "feedback":
        return <FeedbackSection />;
    }
  };

  const settingsGroups = [
    {
      label: null,
      sections: [
        {
          id: "home",
          label: "Home",
          icon: <Home className="h-4 w-4" />,
          description: "Chat with your screen data",
        },
        {
          id: "pipes",
          label: "Pipes",
          icon: <Workflow className="h-4 w-4" />,
          description: "Scheduled agents on your screen data",
        },
        {
          id: "account",
          label: "Account",
          icon: <User className="h-4 w-4" />,
          description: "User account and authentication",
        },
        {
          id: "recording",
          label: "Recording",
          icon: <Video className="h-4 w-4" />,
          description: "Screen and audio recording settings",
        },
        {
          id: "ai",
          label: "AI Settings",
          icon: <Brain className="h-4 w-4" />,
          description: "AI models and processing options",
        },
      ],
    },
    {
      label: "Collaboration",
      sections: [
        {
          id: "team",
          label: "Team",
          icon: <Users className="h-4 w-4" />,
          description: "Share pipe configs with your team",
        },
        ...(showCloudSync
          ? [
              {
                id: "cloud-sync",
                label: "Cloud Sync",
                icon: <Cloud className="h-4 w-4" />,
                description: "Sync your data across devices",
              },
            ]
          : []),
      ],
    },
    {
      label: "Preferences",
      sections: [
        {
          id: "general",
          label: "General",
          icon: <SettingsIcon className="h-4 w-4" />,
          description: "Basic app settings and preferences",
        },
        {
          id: "shortcuts",
          label: "Shortcuts",
          icon: <Keyboard className="h-4 w-4" />,
          description: "Keyboard shortcuts and hotkeys",
        },
        {
          id: "connections",
          label: "Connections",
          icon: <Plug className="h-4 w-4" />,
          description: "Connect to AI assistants like Claude",
        },
        {
          id: "disk-usage",
          label: "Disk Usage",
          icon: <HardDrive className="h-4 w-4" />,
          description: "Monitor storage usage for Screenpipe data",
        },
      ],
    },
    {
      label: null,
      sections: [
        {
          id: "feedback",
          label: "Send Feedback",
          icon: <MessageSquare className="h-4 w-4" />,
          description: "Report issues or share suggestions",
        },
      ],
    },
  ];

  // Listen for navigation events from other windows
  useEffect(() => {
    const unlisten = listen<{ url: string }>("navigate", (event) => {
      const url = new URL(event.payload.url, window.location.origin);
      const section = url.searchParams.get("section");
      if (section && ["home", "general", "ai", "shortcuts", "recording", "account", "disk-usage", "connections", "cloud-sync", "pipes", "team", "feedback"].includes(section)) {
        setActiveSection(section as SettingsSection);
      }
    });

    return () => {
      unlisten.then((unlistenFn) => unlistenFn());
    };
  }, [setActiveSection]);

  return (
    <div className="min-h-screen bg-background">
      {/* Transparent titlebar area */}
      <div className="h-8 bg-transparent" data-tauri-drag-region></div>
      
      <div className="max-w-7xl mx-auto px-4">
        <div className="flex h-[calc(100vh-2rem)] min-h-0">
          {/* Sidebar */}
          <div className="w-56 border-r bg-background flex flex-col min-h-0 rounded-tl-lg">
            <div className="px-4 py-3 border-b">
              <h1 className="text-lg font-bold text-foreground">Settings</h1>
            </div>

            {/* Navigation */}
            <div className="p-2 space-y-1 flex-1 overflow-y-auto">
              {settingsGroups.map((group, gi) => (
                <div key={gi}>
                  {group.label && (
                    <div className="px-3 pt-3 pb-1">
                      <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground/60">
                        {group.label}
                      </span>
                    </div>
                  )}
                  <div className="space-y-0.5">
                    {group.sections.map((section) => (
                      <button
                        key={section.id}
                        onClick={() => setActiveSection(section.id as SettingsSection)}
                        className={cn(
                          "w-full flex items-center space-x-2.5 px-3 py-2 rounded-lg transition-all duration-200 text-left group",
                          activeSection === section.id
                            ? "bg-card shadow-sm border border-border text-foreground"
                            : "hover:bg-card/50 text-muted-foreground hover:text-foreground",
                        )}
                      >
                        <div className={cn(
                          "transition-colors flex-shrink-0",
                          activeSection === section.id
                            ? "text-primary"
                            : "text-muted-foreground group-hover:text-foreground"
                        )}>
                          {section.icon}
                        </div>
                        <span className="font-medium text-sm truncate">{section.label}</span>
                      </button>
                    ))}
                  </div>
                </div>
              ))}
            </div>
            
            {/* Footer */}
            <div className="p-3 border-t border-border flex-shrink-0">
              <div className="text-xs text-muted-foreground text-center">
                screenpipe
              </div>
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 flex flex-col h-full bg-background min-h-0 rounded-tr-lg">
            {activeSection === "home" ? (
              <div className="flex-1 min-h-0 overflow-hidden">
                {renderSection()}
              </div>
            ) : (
              <div className="flex-1 overflow-y-auto overflow-x-hidden min-h-0">
                <div className="p-6 pb-12 max-w-4xl mx-auto">
                  {renderSection()}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export default function SettingsPage() {
  return (
    <Suspense fallback={<div className="min-h-screen bg-background flex items-center justify-center">
      <div className="text-muted-foreground">Loading settings...</div>
    </div>}>
      <SettingsPageContent />
    </Suspense>
  );
} 