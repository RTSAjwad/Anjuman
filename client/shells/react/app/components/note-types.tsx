"use client"

import { useEffect, useState } from "react"
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
import { Tabs, TabsList, TabsTrigger, TabsContent } from "~/components/ui/tabs"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog"
import { ChevronRight } from "lucide-react"

type ScreenProps = {
  noteTypes: NoteTypeView[]
  selected: NoteTypeView | null
  error: string | null
  canManage: boolean
  update: (e: Event) => void
}

export function NoteTypesScreen({ noteTypes, selected, error, canManage, update }: ScreenProps) {
  // The selected template is a client concern (the core tracks the selected note
  // type only). Default to the note type's first template when it changes.
  const [selectedTemplateId, setSelectedTemplateId] = useState<bigint | null>(null)

  useEffect(() => {
    if (!selected) {
      setSelectedTemplateId(null)
      return
    }
    const first = selected.templates[0]
    setSelectedTemplateId(first ? first.id : null)
  }, [selected?.id])

  const selectedTemplate =
    selectedTemplateId !== null
      ? (selected?.templates.find((t) => t.id === selectedTemplateId) ?? null)
      : null

  function selectNoteType(nt: NoteTypeView) {
    update(eventNoteTypeSelect(nt.id))
    setSelectedTemplateId(nt.templates[0]?.id ?? null)
  }

  function selectTemplate(nt: NoteTypeView, templateId: bigint) {
    update(eventNoteTypeSelect(nt.id))
    setSelectedTemplateId(templateId)
  }

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      {/* Sidebar: note types as parents, templates as children */}
      <aside className="w-64 shrink-0 overflow-y-auto border-r p-3">
        {canManage ? <CreateNoteTypeDialog update={update} /> : null}
        {error ? <p className="mt-2 text-xs text-destructive">{error}</p> : null}
        <ul className="mt-3 flex flex-col gap-0.5">
          {noteTypes.map((nt) => (
            <li key={String(nt.id)}>
              <button
                type="button"
                onClick={() => selectNoteType(nt)}
                className={cn(
                  "flex w-full items-center justify-between rounded-md px-2 py-1.5 text-left text-sm hover:bg-muted",
                  selected?.id === nt.id && "bg-accent text-accent-foreground hover:bg-accent",
                )}
              >
                <span className="truncate">{nt.name}</span>
                <span className="text-xs text-muted-foreground">{Number(nt.note_count)}</span>
              </button>
              {nt.templates.length > 0 ? (
                <ul className="ml-4 flex flex-col gap-0.5 border-l border-border pl-2">
                  {nt.templates.map((t) => (
                    <li key={String(t.id)}>
                      <button
                        type="button"
                        onClick={() => selectTemplate(nt, t.id)}
                        className={cn(
                          "flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-left text-xs hover:bg-muted",
                          selected?.id === nt.id && selectedTemplateId === t.id && "bg-accent text-accent-foreground hover:bg-accent",
                        )}
                      >
                        <ChevronRight className="size-3 text-muted-foreground" />
                        <span className="truncate">{t.name}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              ) : null}
            </li>
          ))}
        </ul>
      </aside>

      {/* Panel 1: note type CRUD */}
      <section className="w-72 shrink-0 overflow-y-auto border-r p-4">
        {selected ? (
          <NoteTypePanel key={String(selected.id)} noteType={selected} canManage={canManage} update={update} />
        ) : (
          <p className="text-muted-foreground">Select a note type.</p>
        )}
      </section>

      {/* Panel 3: template CRUD (Styling / Front / Back) */}
      <section className="flex min-h-0 flex-1 flex-col overflow-y-auto p-4">
        {selected ? (
          <TemplatePanel
            noteType={selected}
            template={selectedTemplate}
            selectedTemplateId={selectedTemplateId}
            canManage={canManage}
            update={update}
          />
        ) : (
          <p className="text-muted-foreground">Select a note type.</p>
        )}
      </section>
    </div>
  )
}

// Panel 1 — note-type CRUD (name, fields, sort field) + actions.
function NoteTypePanel({ noteType, canManage, update }: { noteType: NoteTypeView; canManage: boolean; update: (e: Event) => void }) {
  const [name, setName] = useState(noteType.name)
  const [fieldNames, setFieldNames] = useState(noteType.field_names.join(", "))
  const [sortField, setSortField] = useState(noteType.sort_field)
  const [deleteOpen, setDeleteOpen] = useState(false)

  const dirty =
    name !== noteType.name ||
    fieldNames !== noteType.field_names.join(", ") ||
    sortField !== noteType.sort_field

  function save() {
    const fields = fieldNames.split(",").map((f) => f.trim()).filter((f) => f !== "")
    update(eventNoteTypeUpdate(noteType.id, name.trim() === "" ? null : name.trim(), fields, sortField === "" ? "" : sortField, null))
  }

  return (
    <div className="flex flex-col gap-4">
      <h2 className="text-lg font-semibold">{noteType.name}</h2>

      <div className="flex flex-col gap-3">
        <div className="flex flex-col gap-1.5">
          <Label className="text-xs text-muted-foreground">Name</Label>
          <Input value={name} onChange={(e) => setName(e.target.value)} disabled={!canManage} />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label className="text-xs text-muted-foreground">Fields (comma-separated)</Label>
          <Textarea value={fieldNames} onChange={(e) => setFieldNames(e.target.value)} disabled={!canManage} rows={4} />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label className="text-xs text-muted-foreground">Sort field</Label>
          <Input value={sortField} onChange={(e) => setSortField(e.target.value)} disabled={!canManage} placeholder="first field (default)" />
        </div>
      </div>

      {canManage ? (
        <div className="flex flex-wrap gap-2">
          <Button onClick={save} disabled={!dirty}>Save</Button>
          <Button variant="outline" onClick={() => update(eventNoteTypeClone(noteType.id))}>Duplicate</Button>
          <Button variant="destructive" onClick={() => setDeleteOpen(true)}>Delete</Button>
        </div>
      ) : null}

      <DeleteNoteTypeDialog noteType={noteType} open={deleteOpen} onOpenChange={setDeleteOpen} update={update} />
    </div>
  )
}

// Panel 3 — template CRUD with Styling / Front / Back tabs.
function TemplatePanel({ noteType, template, selectedTemplateId, canManage, update }: { noteType: NoteTypeView; template: TemplateView | null; selectedTemplateId: bigint | null; canManage: boolean; update: (e: Event) => void }) {
  const [styling, setStyling] = useState(noteType.styling)
  const [front, setFront] = useState(template?.front_pattern ?? "")
  const [back, setBack] = useState(template?.back_pattern ?? "")

  // Re-seed local draft values when the target changes.
  useEffect(() => {
    setStyling(noteType.styling)
  }, [noteType.id, noteType.styling])
  useEffect(() => {
    setFront(template?.front_pattern ?? "")
    setBack(template?.back_pattern ?? "")
  }, [template?.id])

  const stylingDirty = styling !== noteType.styling
  const frontDirty = template !== null && front !== template.front_pattern
  const backDirty = template !== null && back !== template.back_pattern

  function saveStyling() {
    update(eventNoteTypeUpdate(noteType.id, null, null, null, styling))
  }
  function saveFront() {
    if (template) update(eventNoteTypeTemplateUpdate(noteType.id, template.id, null, front, null))
  }
  function saveBack() {
    if (template) update(eventNoteTypeTemplateUpdate(noteType.id, template.id, null, null, back))
  }

  function moveTemplate(index: number, dir: -1 | 1) {
    const ids = noteType.templates.map((t) => t.id)
    const j = index + dir
    if (j < 0 || j >= ids.length) return
    ;[ids[index], ids[j]] = [ids[j], ids[index]]
    update(eventNoteTypeTemplatesReorder(noteType.id, ids))
  }

  const templateIndex = template ? noteType.templates.findIndex((t) => t.id === template.id) : -1

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">{template ? template.name : "Template"}</h2>
        {canManage ? (
          <div className="flex gap-1">
            {template ? (
              <>
                <Button variant="ghost" size="icon-sm" aria-label="Move up" disabled={templateIndex <= 0} onClick={() => moveTemplate(templateIndex, -1)}>↑</Button>
                <Button variant="ghost" size="icon-sm" aria-label="Move down" disabled={templateIndex < 0 || templateIndex >= noteType.templates.length - 1} onClick={() => moveTemplate(templateIndex, 1)}>↓</Button>
                <Button variant="ghost" size="icon-sm" aria-label="Delete template" onClick={() => update(eventNoteTypeTemplateDelete(noteType.id, template.id))}>✕</Button>
              </>
            ) : null}
            <CreateTemplateDialog noteType={noteType} update={update} disabled={!canManage} />
          </div>
        ) : null}
      </div>

      <Tabs defaultValue="styling" className="flex flex-col gap-3">
        <TabsList>
          <TabsTrigger value="styling">Styling</TabsTrigger>
          <TabsTrigger value="front">Front</TabsTrigger>
          <TabsTrigger value="back">Back</TabsTrigger>
        </TabsList>

        <TabsContent value="styling" className="flex flex-col gap-1.5">
          <Label className="text-xs text-muted-foreground">Styling (CSS, shared by all templates)</Label>
          <Textarea value={styling} onChange={(e) => setStyling(e.target.value)} disabled={!canManage} rows={16} className="font-mono text-xs" />
          {canManage ? <Button onClick={saveStyling} disabled={!stylingDirty}>Save styling</Button> : null}
        </TabsContent>

        <TabsContent value="front" className="flex flex-col gap-1.5">
          <Label className="text-xs text-muted-foreground">Front template — use {"{{Field}}"} placeholders</Label>
          <Textarea value={front} onChange={(e) => setFront(e.target.value)} disabled={!canManage || !template} rows={16} className="font-mono text-xs" />
          {canManage && template ? <Button onClick={saveFront} disabled={!frontDirty}>Save front</Button> : null}
        </TabsContent>

        <TabsContent value="back" className="flex flex-col gap-1.5">
          <Label className="text-xs text-muted-foreground">Back template — use {"{{Field}}"} placeholders</Label>
          <Textarea value={back} onChange={(e) => setBack(e.target.value)} disabled={!canManage || !template} rows={16} className="font-mono text-xs" />
          {canManage && template ? <Button onClick={saveBack} disabled={!backDirty}>Save back</Button> : null}
        </TabsContent>
      </Tabs>
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

function CreateTemplateDialog({ noteType, update, disabled }: { noteType: NoteTypeView; update: (e: Event) => void; disabled: boolean }) {
  const [open, setOpen] = useState(false)
  const [name, setName] = useState("")
  const [front, setFront] = useState("")
  const [back, setBack] = useState("")

  function submit() {
    const n = name.trim()
    if (n === "") return
    update(eventNoteTypeTemplateCreate(noteType.id, n, front, back))
    setName("")
    setFront("")
    setBack("")
    setOpen(false)
  }

  return (
    <>
      <Button variant="outline" size="sm" disabled={disabled} onClick={() => setOpen(true)}>Add template</Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Add template</DialogTitle>
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
