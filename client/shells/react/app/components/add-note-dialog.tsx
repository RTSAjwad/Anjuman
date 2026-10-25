"use client"

import { useState } from "react"
import type { DeckSummary, Event, NoteTypeView } from "shared_types/app"
import { eventNoteCreate } from "shared_types/app"
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

// Flatten the deck tree into { id, path } with full `::`-joined ancestor paths
// (matching the browser's Deck column and the change-deck picker).
function flatDecks(decks: DeckSummary[]): { id: bigint; path: string }[] {
  const out: { id: bigint; path: string }[] = []
  function walk(ds: DeckSummary[], path: string[]) {
    for (const d of ds) {
      const next = [...path, d.title]
      out.push({ id: d.id, path: next.join("::") })
      walk(d.children, next)
    }
  }
  walk(decks, [])
  return out
}

type AddNoteDialogProps = {
  noteTypes: NoteTypeView[]
  decks: DeckSummary[]
  update: (e: Event) => void
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function AddNoteDialog({ noteTypes, decks, update, open, onOpenChange }: AddNoteDialogProps) {
  const [noteTypeId, setNoteTypeId] = useState("")
  const [deckId, setDeckId] = useState("")
  const [fieldValues, setFieldValues] = useState<Record<string, string>>({})

  const noteType = noteTypes.find((nt) => String(nt.id) === noteTypeId) ?? null

  function selectNoteType(id: string) {
    setNoteTypeId(id)
    // Reset field values against the newly-selected note type's field names.
    const nt = noteTypes.find((n) => String(n.id) === id)
    setFieldValues(Object.fromEntries((nt?.field_names ?? []).map((f) => [f, ""])))
  }

  function setField(name: string, value: string) {
    setFieldValues((prev) => ({ ...prev, [name]: value }))
  }

  function submit() {
    if (!noteType || deckId === "") return
    const fields = new Map<string, string>(
      (noteType.field_names ?? []).map((name) => [name, fieldValues[name] ?? ""]),
    )
    update(eventNoteCreate(noteType.id, BigInt(deckId), fields))
    // Reset + close on a valid submit (the core refreshes the notes list).
    setNoteTypeId("")
    setDeckId("")
    setFieldValues({})
    onOpenChange(false)
  }

  const canSubmit = noteType !== null && deckId !== ""

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Add note</DialogTitle>
          <DialogDescription>Create a new note in a deck.</DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1.5">
            <Label className="text-xs text-muted-foreground">Note type</Label>
            <Select
              items={noteTypes.map((nt) => ({ value: String(nt.id), label: nt.name }))}
              value={noteTypeId}
              onValueChange={(v) => { if (v !== null) selectNoteType(v) }}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {noteTypes.map((nt) => (
                    <SelectItem key={String(nt.id)} value={String(nt.id)}>{nt.name}</SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>

          <div className="flex flex-col gap-1.5">
            <Label className="text-xs text-muted-foreground">Deck</Label>
            <Select
              items={flatDecks(decks).map((d) => ({ value: String(d.id), label: d.path }))}
              value={deckId}
              onValueChange={(v) => { if (v !== null) setDeckId(v) }}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {flatDecks(decks).map((d) => (
                    <SelectItem key={String(d.id)} value={String(d.id)}>{d.path}</SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>

          {noteType ? (
            <div className="flex flex-col gap-2">
              {noteType.field_names.map((name) => (
                <div key={name} className="flex flex-col gap-1">
                  <Label className="text-xs text-muted-foreground">{name}</Label>
                  <Input
                    value={fieldValues[name] ?? ""}
                    onChange={(e) => setField(name, e.target.value)}
                  />
                </div>
              ))}
            </div>
          ) : null}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button onClick={submit} disabled={!canSubmit}>Create</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
