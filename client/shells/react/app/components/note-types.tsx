"use client"

import { useState } from "react"
import type { NoteTypeView, TemplateView, Event } from "shared_types/app"
import {
  eventNoteTypeSelect,
  eventNoteTypeCreate,
  eventNoteTypeClone,
  eventNoteTypeUpdate,
  eventNoteTypeDelete,
  eventNoteTypeTemplateCreate,
  eventNoteTypeTemplateUpdate,
  eventNoteTypeTemplateDelete,
  eventNoteTypeTemplatesReorder,
} from "shared_types/app"
import { Button } from "~/components/ui/button"
import { Input } from "~/components/ui/input"
import { Textarea } from "~/components/ui/textarea"
import { Label } from "~/components/ui/label"
import { cn } from "~/lib/utils"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog"

type ScreenProps = {
  noteTypes: NoteTypeView[]
  selected: NoteTypeView | null
  error: string | null
  canManage: boolean
  update: (e: Event) => void
}

export function NoteTypesScreen({ noteTypes, selected, error, canManage, update }: ScreenProps) {
  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      <aside className="w-64 shrink-0 overflow-y-auto border-r p-3">
        {canManage ? (
          <CreateNoteTypeDialog update={update} />
        ) : null}
        {error ? <p className="mt-2 text-xs text-destructive">{error}</p> : null}
        <ul className="mt-3 flex flex-col gap-0.5">
          {noteTypes.map((nt) => (
            <li key={String(nt.id)}>
              <button
                type="button"
                onClick={() => update(eventNoteTypeSelect(nt.id))}
                className={cn(
                  "flex w-full items-center justify-between rounded-md px-2 py-1.5 text-left text-sm hover:bg-muted",
                  selected?.id === nt.id && "bg-accent text-accent-foreground hover:bg-accent",
                )}
              >
                <span className="truncate">{nt.name}</span>
                <span className="text-xs text-muted-foreground">{Number(nt.note_count)}</span>
              </button>
            </li>
          ))}
        </ul>
      </aside>

      <section className="flex min-h-0 flex-1 flex-col overflow-y-auto p-4">
        {selected ? (
          <NoteTypeEditor key={String(selected.id)} noteType={selected} canManage={canManage} update={update} />
        ) : (
          <p className="text-muted-foreground">Select a note type.</p>
        )}
      </section>
    </div>
  )
}

function NoteTypeEditor({ noteType, canManage, update }: { noteType: NoteTypeView; canManage: boolean; update: (e: Event) => void }) {
  const [name, setName] = useState(noteType.name)
  const [fieldNames, setFieldNames] = useState(noteType.field_names.join(", "))
  const [sortField, setSortField] = useState(noteType.sort_field)
  const [styling, setStyling] = useState(noteType.styling)

  const [deleteOpen, setDeleteOpen] = useState(false)
  const [templateDialog, setTemplateDialog] = useState<{ mode: "create" } | { mode: "edit"; template: TemplateView } | null>(null)

  function save() {
    const fields = fieldNames.split(",").map((f) => f.trim()).filter((f) => f !== "")
    update(
      eventNoteTypeUpdate(
        noteType.id,
        name.trim() === "" ? null : name.trim(),
        fields,
        sortField === "" ? "" : sortField,
        styling,
      ),
    )
  }

  function moveTemplate(index: number, dir: -1 | 1) {
    const ids = noteType.templates.map((t) => t.id)
    const j = index + dir
    if (j < 0 || j >= ids.length) return
    ;[ids[index], ids[j]] = [ids[j], ids[index]]
    update(eventNoteTypeTemplatesReorder(noteType.id, ids))
  }

  const dirty = name !== noteType.name || fieldNames !== noteType.field_names.join(", ") || sortField !== noteType.sort_field || styling !== noteType.styling

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">{noteType.name}</h2>
        {canManage ? (
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={() => update(eventNoteTypeClone(noteType.id))}>Duplicate</Button>
            <Button variant="destructive" size="sm" onClick={() => setDeleteOpen(true)}>Delete</Button>
          </div>
        ) : null}
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex flex-col gap-1.5">
          <Label className="text-xs text-muted-foreground">Name</Label>
          <Input value={name} onChange={(e) => setName(e.target.value)} disabled={!canManage} />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label className="text-xs text-muted-foreground">Fields (comma-separated)</Label>
          <Input value={fieldNames} onChange={(e) => setFieldNames(e.target.value)} disabled={!canManage} />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label className="text-xs text-muted-foreground">Sort field</Label>
          <Input value={sortField} onChange={(e) => setSortField(e.target.value)} disabled={!canManage} placeholder="first field (default)" />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label className="text-xs text-muted-foreground">Styling (CSS)</Label>
          <Textarea value={styling} onChange={(e) => setStyling(e.target.value)} disabled={!canManage} rows={8} className="font-mono text-xs" />
        </div>
        {canManage ? (
          <Button onClick={save} disabled={!dirty}>Save</Button>
        ) : null}
      </div>

      <div className="flex flex-col gap-2 border-t pt-4">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-medium">Templates</h3>
          {canManage ? (
            <Button variant="outline" size="sm" onClick={() => setTemplateDialog({ mode: "create" })}>Add template</Button>
          ) : null}
        </div>
        {noteType.templates.length === 0 ? (
          <p className="text-sm text-muted-foreground">No templates.</p>
        ) : (
          <ul className="flex flex-col gap-1">
            {noteType.templates.map((t, i) => (
              <li key={String(t.id)} className="flex items-center justify-between gap-2 rounded-md border px-2 py-1.5 text-sm">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="font-medium">{t.name}</span>
                    {canManage ? (
                      <button type="button" className="text-xs text-muted-foreground hover:underline" onClick={() => setTemplateDialog({ mode: "edit", template: t })}>edit</button>
                    ) : null}
                  </div>
                  <div className="truncate text-xs text-muted-foreground">{t.front_pattern}</div>
                </div>
                {canManage ? (
                  <div className="flex shrink-0 items-center gap-1">
                    <Button variant="ghost" size="icon-sm" aria-label="Move up" disabled={i === 0} onClick={() => moveTemplate(i, -1)}>↑</Button>
                    <Button variant="ghost" size="icon-sm" aria-label="Move down" disabled={i === noteType.templates.length - 1} onClick={() => moveTemplate(i, 1)}>↓</Button>
                    <Button variant="ghost" size="icon-sm" aria-label="Delete template" onClick={() => update(eventNoteTypeTemplateDelete(noteType.id, t.id))}>✕</Button>
                  </div>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </div>

      <DeleteNoteTypeDialog
        noteType={noteType}
        open={deleteOpen}
        onOpenChange={setDeleteOpen}
        update={update}
      />

      <TemplateDialog
        noteType={noteType}
        state={templateDialog}
        onClose={() => setTemplateDialog(null)}
        update={update}
      />
    </div>
  )
}

function CreateNoteTypeDialog({ update }: { update: (e: Event) => void }) {
  const [open, setOpen] = useState(false)
  const [name, setName] = useState("")
  const [fields, setFields] = useState("")

  function submit() {
    const n = name.trim()
    if (n === "") return
    const fs = fields.split(",").map((f) => f.trim()).filter((f) => f !== "")
    update(eventNoteTypeCreate(n, fs))
    setName("")
    setFields("")
    setOpen(false)
  }

  return (
    <>
      <Button variant="outline" size="sm" className="w-full" onClick={() => setOpen(true)}>New note type</Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>New note type</DialogTitle>
            <DialogDescription>Define the note type's name and fields.</DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-3">
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs text-muted-foreground">Name</Label>
              <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Basic" />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label className="text-xs text-muted-foreground">Fields (comma-separated)</Label>
              <Input value={fields} onChange={(e) => setFields(e.target.value)} placeholder="Front, Back" />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setOpen(false)}>Cancel</Button>
            <Button onClick={submit} disabled={name.trim() === ""}>Create</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}

function DeleteNoteTypeDialog({ noteType, open, onOpenChange, update }: { noteType: NoteTypeView; open: boolean; onOpenChange: (o: boolean) => void; update: (e: Event) => void }) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete note type</DialogTitle>
          <DialogDescription>
            This deletes the note type, its templates, and all of its notes/cards.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="destructive" onClick={() => { update(eventNoteTypeDelete(noteType.id)); onOpenChange(false); }}>Delete</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function TemplateDialog({ noteType, state, onClose, update }: { noteType: NoteTypeView; state: { mode: "create" } | { mode: "edit"; template: TemplateView } | null; onClose: () => void; update: (e: Event) => void }) {
  const editing = state?.mode === "edit" ? state.template : null
  const [name, setName] = useState(editing?.name ?? "")
  const [front, setFront] = useState(editing?.front_pattern ?? "")
  const [back, setBack] = useState(editing?.back_pattern ?? "")

  // Reset when the dialog target changes.
  const [lastTarget, setLastTarget] = useState<string | null>(null)
  const targetKey = state ? (state.mode === "edit" ? `edit:${state.template.id}` : "create") : null
  if (targetKey !== lastTarget) {
    setLastTarget(targetKey)
    setName(editing?.name ?? "")
    setFront(editing?.front_pattern ?? "")
    setBack(editing?.back_pattern ?? "")
  }

  function submit() {
    if (!state) return
    const n = name.trim()
    if (n === "") return
    if (state.mode === "create") {
      update(eventNoteTypeTemplateCreate(noteType.id, n, front, back))
    } else {
      update(eventNoteTypeTemplateUpdate(noteType.id, state.template.id, n, front, back))
    }
    onClose()
  }

  return (
    <Dialog open={state !== null} onOpenChange={(v) => { if (!v) onClose(); }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{state?.mode === "edit" ? "Edit template" : "Add template"}</DialogTitle>
          <DialogDescription>Use {"{{Field}}"} placeholders for note fields.</DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-3">
          <div className="flex flex-col gap-1.5">
            <Label className="text-xs text-muted-foreground">Name</Label>
            <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="Card 1" />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label className="text-xs text-muted-foreground">Front pattern</Label>
            <Textarea value={front} onChange={(e) => setFront(e.target.value)} rows={3} className="font-mono text-xs" />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label className="text-xs text-muted-foreground">Back pattern</Label>
            <Textarea value={back} onChange={(e) => setBack(e.target.value)} rows={3} className="font-mono text-xs" />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>Cancel</Button>
          <Button onClick={submit} disabled={name.trim() === ""}>Save</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
