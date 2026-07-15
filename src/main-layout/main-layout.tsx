import { useQuery } from "@tanstack/react-query";
import { emptyWorkspaceSnapshot } from "../services/mock-workspace-data";
import { workspaceService } from "../services/runtime-workspace-client";
import { ConversationSidebar } from "./conversation-sidebar";
import { FlowCanvas } from "./flow-canvas";
import { InfoPanel } from "./info-panel";
import { StatusBar } from "./status-bar";
import { TopBar } from "./top-bar";

export function MainLayout({ onOpenSettings }: { onOpenSettings: () => void }) {
  const workspaceQuery = useQuery({
    queryKey: ["workspace", "snapshot"],
    queryFn: () => workspaceService.getWorkspaceSnapshot(),
  });
  const workspace = workspaceQuery.data ?? emptyWorkspaceSnapshot;

  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="pointer-events-none fixed inset-0 opacity-[0.035] [background-image:linear-gradient(hsl(var(--primary))_1px,transparent_1px),linear-gradient(90deg,hsl(var(--primary))_1px,transparent_1px)] [background-size:100px_100px]" />
      <div className="relative flex min-h-screen flex-col">
        <TopBar />
        <div className="grid flex-1 gap-4 p-2 xl:grid-cols-[230px_minmax(0,620px)_minmax(290px,1fr)]">
          <ConversationSidebar conversations={workspace.conversations} onOpenSettings={onOpenSettings} tools={workspace.tools} />
          <FlowCanvas agentNodes={workspace.agentNodes} chatMessages={workspace.chatMessages} />
          <InfoPanel />
        </div>
        <StatusBar />
      </div>
    </main>
  );
}
