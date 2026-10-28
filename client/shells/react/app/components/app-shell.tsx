import { useCore } from "../lib/core-provider.client";
import { useHashRouter } from "../lib/use-hash-router";
import type { DeckSummary, Event, ViewModel } from "shared_types/app";
import {
  eventLogout,
  eventShowDeckList,
  eventShowCards,
  eventShowNotes,
  eventNoteDetailRequested,
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
} from "./ui/sidebar";
import { Separator } from "./ui/separator";
import { Fragment, useEffect, useState } from "react";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "./ui/breadcrumb";
import { Tabs, TabsList, TabsTrigger } from "./ui/tabs";
import { Avatar, AvatarFallback } from "./ui/avatar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { LibraryIcon, TableIcon, LogOutIcon } from "lucide-react";
import {
  DeckListScreen,
  StudyScreen,
  DeckOptionsScreen,
  BrowserScreen,
} from "./screens";
import { CardActions, NoteActions } from "./browser";
import { AddNoteDialog } from "./add-note-dialog";
import type { NotePrefill } from "./add-note-dialog";
import { StudyCountBadges, StudyControls } from "./study";
import { LoginScreen } from "./login-screen";
import { ThemeSelector } from "./theme-selector";

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

  return <ShellView view={view} update={update} />;
}

function ShellView({ view, update }: { view: ViewModel; update: (e: Event) => void }) {
  const [addNoteOpen, setAddNoteOpen] = useState(false);
  const [prefill, setPrefill] = useState<NotePrefill | null>(null);
  const [pendingDuplicate, setPendingDuplicate] = useState(false);

  const route = view.route.kind;
  const canManageNotes = view.permissions.can_manage_notes;
  const canManageTags = view.permissions.can_manage_tags;
  const hasUnusedTags = view.has_unused_tags;
  const authenticated = view.authenticated;

  // Mirror route <-> URL (projection; the core stays the source of truth).
  useHashRouter({
    route: view.route,
    authenticated,
    decksLoaded: view.decks_loaded,
    selectedDeckId: view.selected_deck?.id ?? null,
    onEvent: update,
  });

  // Study-screen duplicate: once `view.study_note` arrives after the user picked
  // "Duplicate Note", prefill + open the create-note dialog. (Must be above the
  // auth guard so the hook count stays stable across the login/logout flip.)
  useEffect(() => {
    if (!pendingDuplicate) return;
    const n = view.study_note;
    if (!n) return;
    setPrefill({
      noteTypeId: String(n.note_type_id),
      deckId: n.deck_id === null ? "" : String(n.deck_id),
      fields: Object.fromEntries(n.fields),
      tags: [...n.tags],
    });
    setAddNoteOpen(true);
    setPendingDuplicate(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view.study_note, pendingDuplicate]);

  if (!authenticated) {
    return <LoginScreen />;
  }

  function openCreateNote() {
    setPrefill(null);
    setAddNoteOpen(true);
  }

  function openDuplicateNote() {
    if (route === "Cards") {
      const c = view.browser_selected;
      if (!c) return;
      setPrefill({
        noteTypeId: String(c.note_type_id),
        deckId: String(c.deck_id),
        fields: Object.fromEntries(c.fields),
        tags: [...c.tags],
      });
    } else {
      const n = view.notes_selected;
      if (!n) return;
      setPrefill({
        noteTypeId: String(n.note_type_id),
        deckId: n.deck_id === null ? "" : String(n.deck_id),
        fields: Object.fromEntries(n.fields),
        tags: [...n.tags],
      });
    }
    setAddNoteOpen(true);
  }

  // Study-screen duplicate: fetch the in-study note's full payload, then open the
  // prefill dialog once `view.study_note` arrives.
  function openDuplicateStudyNote() {
    if (!view.current_card) return;
    update(eventNoteDetailRequested(view.current_card.note_id));
    setPendingDuplicate(true);
  }

  const hasSelection =
    route === "Cards" ? view.browser_selected !== null : view.notes_selected !== null;

  return (
    <div className="flex h-svh w-full">
      <AppSidebar
        displayName={view.display_name}
        email={view.email}
        route={route}
        canManageNotes={canManageNotes}
        onShowDeckList={() => update(eventShowDeckList())}
        onShowCards={() => update(eventShowCards())}
        onLogout={() => update(eventLogout())}
      />
      <SidebarInset className="min-w-0 flex-1">
        <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border bg-background px-3">
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
            <>
              <BrowserTabs
                route={route}
                canManageNotes={canManageNotes}
                onShowCards={() => update(eventShowCards())}
                onShowNotes={() => update(eventShowNotes())}
              />
              <Separator orientation="vertical" />
              <span className="text-sm font-medium text-muted-foreground">Note Actions</span>
              <NoteActions
                noteId={route === "Cards" ? (view.browser_selected?.note_id ?? null) : (view.notes_selected?.id ?? null)}
                noteSuspended={route === "Cards" ? (view.browser_selected?.note_suspended ?? null) : (view.notes_selected?.note_suspended ?? null)}
                noteBuried={route === "Cards" ? (view.browser_selected?.note_buried ?? null) : (view.notes_selected?.note_buried ?? null)}
                canManageNotes={canManageNotes}
                canManageTags={canManageTags}
                hasUnusedTags={hasUnusedTags}
                onCreateNote={openCreateNote}
                onDuplicate={hasSelection ? openDuplicateNote : null}
                update={update}
              />
              {route === "Cards" && view.browser_selected ? (
                <CardActions card={view.browser_selected} decks={view.decks} canManageNotes={canManageNotes} update={update} />
              ) : null}
            </>
          ) : route === "Study" ? (
            <>
              <DeckBreadcrumb
                decks={view.decks}
                selectedDeckId={view.selected_deck?.id ?? null}
                subview="Study"
                onBack={() => update(eventShowDeckList())}
              />
              <StudyCountBadges counts={view.counts} />
              {view.current_card ? (
                <StudyControls
                  card={view.current_card}
                  canManageNotes={canManageNotes}
                  onDuplicateNote={openDuplicateStudyNote}
                  update={update}
                />
              ) : null}
            </>
          ) : (
            <DeckBreadcrumb
              decks={view.decks}
              selectedDeckId={view.selected_deck?.id ?? null}
              subview="Options"
              onBack={() => update(eventShowDeckList())}
            />
          )}
        </header>

        <main className="flex min-h-0 flex-1 flex-col overflow-hidden">
          <Screen route={route} />
        </main>
      </SidebarInset>

      <AddNoteDialog
        noteTypes={view.note_types}
        decks={view.decks}
        update={update}
        open={addNoteOpen}
        onOpenChange={setAddNoteOpen}
        prefill={prefill}
      />
    </div>
  );
}

function AppSidebar({
  displayName,
  email,
  route,
  canManageNotes,
  onShowDeckList,
  onShowCards,
  onLogout,
}: {
  displayName: string;
  email: string;
  route: string;
  canManageNotes: boolean;
  onShowDeckList: () => void;
  onShowCards: () => void;
  onLogout: () => void;
}) {
  return (
    <Sidebar collapsible="icon">
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
            <ThemeSelector />
          </SidebarMenuItem>
          <SidebarMenuItem>
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <SidebarMenuButton
                    size="lg"
                    className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
                  >
                    <Avatar className="size-8 rounded-lg">
                      <AvatarFallback className="rounded-lg">
                        {initials(displayName)}
                      </AvatarFallback>
                    </Avatar>
                    <div className="grid flex-1 text-left text-sm leading-tight">
                      <span className="truncate font-semibold">{displayName}</span>
                      <span className="truncate text-xs text-muted-foreground">
                        {email}
                      </span>
                    </div>
                  </SidebarMenuButton>
                }
              />
              <DropdownMenuContent align="start" className="w-[--aria-trigger-width] min-w-56 rounded-lg">
                <DropdownMenuGroup>
                  <DropdownMenuLabel className="p-0 font-normal">
                    <div className="flex items-center gap-2 px-1 py-1.5 text-left text-sm">
                      <Avatar className="size-8 rounded-lg">
                        <AvatarFallback className="rounded-lg">
                          {initials(displayName)}
                        </AvatarFallback>
                      </Avatar>
                      <div className="grid flex-1 text-left text-sm leading-tight">
                        <span className="truncate font-semibold">{displayName}</span>
                        <span className="truncate text-xs text-muted-foreground">
                          {email}
                        </span>
                      </div>
                    </div>
                  </DropdownMenuLabel>
                </DropdownMenuGroup>
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={onLogout} variant="destructive">
                  <LogOutIcon />
                  Log out
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </SidebarMenuItem>
        </SidebarMenu>
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
    <Tabs
      value={route}
      onValueChange={(v) => {
        if (v === null) return;
        if (v === "Notes") onShowNotes();
        else onShowCards();
      }}
    >
      <TabsList>
        <TabsTrigger value="Cards">Cards</TabsTrigger>
        {canManageNotes ? <TabsTrigger value="Notes">Notes</TabsTrigger> : null}
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

function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? "")
    .join("");
}

// Walk the deck tree to compute the ancestor title chain (root → target, incl.
// the target) for breadcrumbs. Mirrors the Leptos `deck_ancestor_titles`.
function deckAncestorTitles(decks: DeckSummary[], deckId: bigint): string[] {
  function walk(ds: DeckSummary[], path: string[]): string[] | null {
    for (const d of ds) {
      path.push(d.title);
      if (d.id === deckId) return path;
      const found = walk(d.children, path);
      if (found) return found;
      path.pop();
    }
    return null;
  }
  return walk(decks, []) ?? [];
}

function DeckBreadcrumb({
  decks,
  selectedDeckId,
  subview,
  onBack,
}: {
  decks: DeckSummary[];
  selectedDeckId: bigint | null;
  subview: "Study" | "Options";
  onBack: () => void;
}) {
  const titles = selectedDeckId === null ? [] : deckAncestorTitles(decks, selectedDeckId);
  const ownTitle = titles[titles.length - 1];
  const ancestors = titles.slice(0, -1);

  return (
    <Breadcrumb className="flex-1">
      <BreadcrumbList>
        <BreadcrumbItem>
          <button
            type="button"
            onClick={onBack}
            className="text-muted-foreground transition-colors hover:text-foreground"
          >
            Decks
          </button>
        </BreadcrumbItem>
        {ancestors.map((t) => (
          <Fragment key={t}>
            <BreadcrumbSeparator />
            <BreadcrumbItem>
              <span className="text-muted-foreground">{t}</span>
            </BreadcrumbItem>
          </Fragment>
        ))}
        {ownTitle ? (
          <>
            <BreadcrumbSeparator />
            <BreadcrumbItem>
              <span className="text-muted-foreground">{ownTitle}</span>
            </BreadcrumbItem>
          </>
        ) : null}
        <BreadcrumbSeparator />
        <BreadcrumbItem>
          <BreadcrumbPage>{subview}</BreadcrumbPage>
        </BreadcrumbItem>
      </BreadcrumbList>
    </Breadcrumb>
  );
}
