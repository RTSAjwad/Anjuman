"use client"

import { useEffect, useRef, useState } from "react"
import type { DeckDetailView, Event, UserSearchView } from "shared_types/app"
import {
  eventDeckDetailRequested,
  eventDeckShare,
  eventDeckUnshare,
  eventDeckTransferOwner,
  eventUsersSearchRequested,
} from "shared_types/app"
import { Button } from "~/components/ui/button"
import { Input } from "~/components/ui/input"
import { Label } from "~/components/ui/label"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "~/components/ui/select"

function fullName(first: string, last: string): string {
  const n = `${first} ${last}`.trim()
  return n === "" ? "—" : n
}

type Props = {
  deckId: bigint | null
  canManageDecks: boolean
  deckDetail: DeckDetailView | null
  deckDetailError: string | null
  userSearch: UserSearchView[]
  userSearchError: string | null
  update: (e: Event) => void
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function DeckCollaboratorsDialog({
  deckId,
  canManageDecks,
  deckDetail,
  deckDetailError,
  userSearch,
  userSearchError,
  update,
  open,
  onOpenChange,
}: Props) {
  const [query, setQuery] = useState("")
  const [pendingOwner, setPendingOwner] = useState("")
  const debounceRef = useRef<number | null>(null)

  // Fetch the detail whenever the dialog opens for a deck.
  useEffect(() => {
    if (open && deckId !== null) {
      update(eventDeckDetailRequested(deckId))
      setQuery("")
      setPendingOwner("")
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, deckId]);

  // Debounced user search.
  useEffect(() => {
    if (debounceRef.current !== null) window.clearTimeout(debounceRef.current)
    debounceRef.current = window.setTimeout(() => {
      if (open && canManageDecks) {
        update(eventUsersSearchRequested(query.trim()))
      }
    }, 300)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, open, canManageDecks]);

  const collaborators = deckDetail?.collaborators ?? []
  const classes = deckDetail?.classes ?? []

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Manage deck</DialogTitle>
          <DialogDescription>Owner, maintainers, and shared classes.</DialogDescription>
        </DialogHeader>

        {deckDetailError ? (
          <p className="text-sm text-destructive">{deckDetailError}</p>
        ) : null}

        <div className="flex flex-col gap-4">
          {/* Owner */}
          <div className="flex flex-col gap-1">
            <Label className="text-xs text-muted-foreground">Owner</Label>
            {deckDetail ? (
              <div className="text-sm">
                <span className="font-medium">
                  {fullName(deckDetail.owner_first_name, deckDetail.owner_last_name)}
                </span>{" "}
                <span className="text-muted-foreground">{deckDetail.owner_email}</span>
              </div>
            ) : (
              <span className="text-sm text-muted-foreground">Loading…</span>
            )}
          </div>

          {/* Collaborators */}
          <div className="flex flex-col gap-1.5">
            <Label className="text-xs text-muted-foreground">Maintainers</Label>
            {collaborators.length === 0 ? (
              <span className="text-sm text-muted-foreground">None</span>
            ) : (
              <ul className="flex flex-col gap-1">
                {collaborators.map((c) => (
                  <li key={String(c.user_id)} className="flex items-center justify-between gap-2 text-sm">
                    <span>
                      <span className="font-medium">{fullName(c.first_name, c.last_name)}</span>{" "}
                      <span className="text-muted-foreground">{c.email}</span>
                    </span>
                    {canManageDecks && deckId !== null ? (
                      <Button
                        variant="outline"
                        size="xs"
                        onClick={() => update(eventDeckUnshare(deckId, c.user_id))}
                      >
                        Unshare
                      </Button>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
          </div>

          {/* Classes */}
          <div className="flex flex-col gap-1.5">
            <Label className="text-xs text-muted-foreground">Classes</Label>
            {classes.length === 0 ? (
              <span className="text-sm text-muted-foreground">None</span>
            ) : (
              <ul className="flex flex-col gap-1">
                {classes.map((c) => (
                  <li key={String(c.id)} className="text-sm">{c.name}</li>
                ))}
              </ul>
            )}
          </div>

          {/* Manage: share + transfer */}
          {canManageDecks && deckId !== null ? (
            <div className="flex flex-col gap-3 border-t pt-3">
              <div className="flex flex-col gap-2">
                <Label className="text-xs text-muted-foreground">Share with a maintainer</Label>
                <Input
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder="Search teachers…"
                />
                {userSearchError ? (
                  <p className="text-xs text-destructive">{userSearchError}</p>
                ) : null}
                {userSearch.length > 0 ? (
                  <ul className="flex max-h-40 flex-col gap-0.5 overflow-y-auto">
                    {userSearch.map((u) => (
                      <li key={String(u.id)}>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="w-full justify-start"
                          onClick={() => update(eventDeckShare(deckId, u.id))}
                        >
                          {fullName(u.first_name, u.last_name)} — {u.email}
                        </Button>
                      </li>
                    ))}
                  </ul>
                ) : null}
              </div>

              <div className="flex flex-col gap-1.5">
                <Label className="text-xs text-muted-foreground">Transfer ownership</Label>
                <Select
                  items={userSearch.map((u) => ({ value: String(u.id), label: `${fullName(u.first_name, u.last_name)} (${u.email})` }))}
                  value={pendingOwner}
                  onValueChange={(v) => { if (v !== null) setPendingOwner(v) }}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select a teacher" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      {userSearch.map((u) => (
                        <SelectItem key={String(u.id)} value={String(u.id)}>
                          {fullName(u.first_name, u.last_name)} ({u.email})
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
                <Button
                  variant="outline"
                  onClick={() => {
                    if (pendingOwner !== "") {
                      update(eventDeckTransferOwner(deckId, BigInt(pendingOwner)))
                      setPendingOwner("")
                    }
                  }}
                  disabled={pendingOwner === ""}
                >
                  Transfer
                </Button>
              </div>
            </div>
          ) : null}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Close</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
