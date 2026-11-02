"use client"

import { useEffect, useState } from "react"
import type { DeckSummary, Event } from "shared_types/app"
import { eventDeckCreate } from "shared_types/app"
import { Button } from "~/components/ui/button"
import { Input } from "~/components/ui/input"
import { Textarea } from "~/components/ui/textarea"
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

// Flatten the deck tree into { id, path } with full `::`-joined ancestor paths,
// matching the deck-column / change-deck / add-note pickers.
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

type CreateDeckProps = {
  decks: DeckSummary[]
  update: (e: Event) => void
}

export function CreateDeck({ decks, update }: CreateDeckProps) {
  const [open, setOpen] = useState(false)
  const [title, setTitle] = useState("")
  const [description, setDescription] = useState("")
  const [parentId, setParentId] = useState("")

  // Reset the form each time the dialog opens.
  useEffect(() => {
    if (open) {
      setTitle("")
      setDescription("")
      setParentId("")
    }
  }, [open])

  function submit() {
    const t = title.trim()
    if (t === "") return
    update(
      eventDeckCreate(
        t,
        description.trim() === "" ? null : description.trim(),
        parentId === "" ? null : BigInt(parentId),
      ),
    )
    setOpen(false)
  }

  return (
    <>
      <Button variant="outline" onClick={() => setOpen(true)}>
        New deck
      </Button>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>New deck</DialogTitle>
            <DialogDescription>Create a deck at the top level or under a parent.</DialogDescription>
          </DialogHeader>

          <div className="flex flex-col gap-4">
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs text-muted-foreground">Title</Label>
              <Input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder="e.g. Biology 101"
              />
            </div>

            <div className="flex flex-col gap-1.5">
              <Label className="text-xs text-muted-foreground">Description</Label>
              <Textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Optional"
              />
            </div>

            <div className="flex flex-col gap-1.5">
              <Label className="text-xs text-muted-foreground">Parent</Label>
              <Select
                items={[{ value: "", label: "No parent" }, ...flatDecks(decks).map((d) => ({ value: String(d.id), label: d.path }))]}
                value={parentId}
                onValueChange={(v) => { if (v !== null) setParentId(v) }}
              >
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="No parent" />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    <SelectItem value="">No parent</SelectItem>
                    {flatDecks(decks).map((d) => (
                      <SelectItem key={String(d.id)} value={String(d.id)}>{d.path}</SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setOpen(false)}>Cancel</Button>
            <Button onClick={submit} disabled={title.trim() === ""}>Create</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
