import { useCore } from "../lib/core-provider.client";
import { useHashRouter } from "../lib/use-hash-router";
import {
  eventLogout,
  eventShowDeckList,
  eventShowCards,
  eventShowNotes,
} from "shared_types/app";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarTrigger,
  SidebarHeader,
} from "./ui/sidebar";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbList,
  BreadcrumbPage,
} from "./ui/breadcrumb";
import { Tabs, TabsList, TabsTrigger } from "./ui/tabs";
import { LibraryIcon, TableIcon, LogOutIcon } from "lucide-react";
import {
  DeckListScreen,
  StudyScreen,
  DeckOptionsScreen,
  BrowserScreen,
} from "./screens";
import { LoginScreen } from "./login-screen";

export function AppShell() {
  return (
    <SidebarProvider defaultOpen={false}>
      <Layout />
    </SidebarProvider>
  );
}

function Layout() {
  const { view, update } = useCore();

  if (view === null) {
    return <p className="p-4">Loading…</p>;
  }

  const route = view.route.kind;
  const canManageNotes = view.permissions.can_manage_notes;
  const authenticated = view.authenticated;

  // Mirror route <-> URL (projection; the core stays the source of truth).
  useHashRouter({
    route: view.route,
    authenticated,
    decksLoaded: view.decks.length > 0,
    selectedDeckId: view.selected_deck?.id ?? null,
    onEvent: update,
  });

  if (!authenticated) {
    return <LoginScreen />;
  }

  return (
    <div className="flex h-svh w-full">
      <AppSidebar
        displayName={view.display_name}
        route={route}
        onShowDeckList={() => update(eventShowDeckList())}
        onShowCards={() => update(eventShowCards())}
        onLogout={() => update(eventLogout())}
      />
      <SidebarInset className="min-w-0 flex-1">
        <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border bg-muted px-3">
          <SidebarTrigger />
          {route === "DeckList" ? (
            <Breadcrumb>
              <BreadcrumbList>
                <BreadcrumbItem>
                  <BreadcrumbPage>Decks</BreadcrumbPage>
                </BreadcrumbItem>
              </BreadcrumbList>
            </Breadcrumb>
          ) : route === "Cards" || route === "Notes" ? (
            <BrowserTabs
              route={route}
              canManageNotes={canManageNotes}
              onShowCards={() => update(eventShowCards())}
              onShowNotes={() => update(eventShowNotes())}
            />
          ) : (
            // Study / DeckOptions — placeholder breadcrumb (full in Phase 3).
            <Breadcrumb>
              <BreadcrumbList>
                <BreadcrumbItem>
                  <BreadcrumbPage>
                    {route === "Study" ? "Study" : "Options"}
                  </BreadcrumbPage>
                </BreadcrumbItem>
              </BreadcrumbList>
            </Breadcrumb>
          )}
        </header>

        <main className="flex min-h-0 flex-1 flex-col overflow-hidden">
          <Screen route={route} />
        </main>
      </SidebarInset>
    </div>
  );
}

function AppSidebar({
  displayName,
  route,
  onShowDeckList,
  onShowCards,
  onLogout,
}: {
  displayName: string;
  route: string;
  onShowDeckList: () => void;
  onShowCards: () => void;
  onLogout: () => void;
}) {
  return (
    <Sidebar collapsible="icon">
      <SidebarHeader className="px-3 py-3 font-semibold">Anjuman</SidebarHeader>
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              <SidebarMenuItem>
                <SidebarMenuButton
                  onClick={onShowDeckList}
                  isActive={route === "DeckList"}
                  tooltip="Decks"
                >
                  <LibraryIcon />
                  <span>Decks</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
              <SidebarMenuItem>
                <SidebarMenuButton
                  onClick={onShowCards}
                  isActive={route === "Cards" || route === "Notes"}
                  tooltip="Browser"
                >
                  <TableIcon />
                  <span>Browser</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton onClick={onLogout} tooltip="Log out">
              <LogOutIcon />
              <span>Log out</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
        <div className="px-2 pb-2 text-xs text-muted-foreground">{displayName}</div>
      </SidebarFooter>
    </Sidebar>
  );
}

function BrowserTabs({
  route,
  canManageNotes,
  onShowCards,
  onShowNotes,
}: {
  route: string;
  canManageNotes: boolean;
  onShowCards: () => void;
  onShowNotes: () => void;
}) {
  return (
    <Tabs value={route} onValueChange={(v) => (v === "notes" ? onShowNotes() : onShowCards())}>
      <TabsList>
        <TabsTrigger value="cards">Cards</TabsTrigger>
        {canManageNotes ? <TabsTrigger value="notes">Notes</TabsTrigger> : null}
      </TabsList>
    </Tabs>
  );
}

function Screen({ route }: { route: string }) {
  switch (route) {
    case "DeckList":
      return <DeckListScreen />;
    case "Cards":
    case "Notes":
      return <BrowserScreen />;
    case "Study":
      return <StudyScreen />;
    case "DeckOptions":
      return <DeckOptionsScreen />;
    default:
      return <DeckListScreen />;
  }
}
