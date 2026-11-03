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
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "~/components/ui/dropdown-menu"
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "~/components/ui/collapsible"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog"
import { ChevronRight, ChevronUp, ChevronDown, X, EllipsisVertical } from "lucide-react"

type ScreenProps = {
  noteTypes: NoteTypeView[]
  selected: NoteTypeView | null
  error: string | null
  canManage: boolean
  update: (e: Event) => void
}

export function NoteTypesScreen({ noteTypes, selected, error, canManage, update }: ScreenProps) {
  const [selectedTemplateId, setSelectedTemplateId] = useState<bigint | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<NoteTypeView | null>(null)
  const [addTemplateTarget, setAddTemplateTarget] = useState<NoteTypeView | null>(null)

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

  function moveTemplate(nt: NoteTypeView, index: number, dir: -1 | 1) {
    const ids = nt.templates.map((t) => t.id)
    const j = index + dir
    if (j < 0 || j >= ids.length) return
    ;[ids[index], ids[j]] = [ids[j], ids[index]]
    update(eventNoteTypeTemplatesReorder(nt.id, ids))
  }

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      {/* Sidebar: note types as parents, templates as children with their own actions */}
      <aside className="w-64 shrink-0 overflow-y-auto border-r p-3">
        {canManage ? <CreateNoteTypeDialog update={update} /> : null}
        {error ? <p className="mt-2 text-xs text-destructive">{error}</p> : null}
        <ul className="mt-3 flex flex-col">
          {noteTypes.map((nt) => (
            <NoteTypeTreeNode
              key={String(nt.id)}
              noteType={nt}
              selectedId={selected?.id ?? null}
              selectedTemplateId={selectedTemplateId}
              canManage={canManage}
              onSelectNoteType={selectNoteType}
              onSelectTemplate={selectTemplate}
              onAddTemplate={() => setAddTemplateTarget(nt)}
              onDuplicate={() => update(eventNoteTypeClone(nt.id))}
              onDelete={() => setDeleteTarget(nt)}
              onMoveTemplate={(index, dir) => moveTemplate(nt, index, dir)}
              onDeleteTemplate={(templateId) => update(eventNoteTypeTemplateDelete(nt.id, templateId))}
            />
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

      {/* Panel 3: template CRUD (tabs on top, textarea fills, save at bottom) */}
      <section className="flex min-h-0 flex-1 flex-col overflow-hidden">
        {selected ? (
          <TemplatePanel
            noteType={selected}
            template={selectedTemplate}
            canManage={canManage}
            update={update}
          />
        ) : (
          <p className="p-4 text-muted-foreground">Select a note type.</p>
        )}
      </section>

      <DeleteNoteTypeDialog noteType={deleteTarget} open={deleteTarget !== null} onOpenChange={(v) => { if (!v) setDeleteTarget(null) }} update={update} />
      <CreateTemplateDialog noteType={addTemplateTarget} update={update} open={addTemplateTarget !== null} onOpenChange={(v) => { if (!v) setAddTemplateTarget(null) }} />
    </div>
  )
}

// Sidebar tree node: note type (chevron + dropdown) → templates (inline actions).
function NoteTypeTreeNode({
  noteType,
  selectedId,
  selectedTemplateId,
  canManage,
  onSelectNoteType,
  onSelectTemplate,
  onAddTemplate,
  onDuplicate,
  onDelete,
  onMoveTemplate,
  onDeleteTemplate,
}: {
  noteType: NoteTypeView
  selectedId: bigint | null
  selectedTemplateId: bigint | null
  canManage: boolean
  onSelectNoteType: (nt: NoteTypeView) => void
  onSelectTemplate: (nt: NoteTypeView, templateId: bigint) => void
  onAddTemplate: () => void
  onDuplicate: () => void
  onDelete: () => void
  onMoveTemplate: (index: number, dir: -1 | 1) => void
  onDeleteTemplate: (templateId: bigint) => void
}) {
  const [open, setOpen] = useState(true)
  const hasChildren = noteType.templates.length > 0
  const isSelected = selectedId === noteType.id

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <li className="flex flex-col">
        <div className="group/row flex items-center">
          {hasChildren ? (
            <CollapsibleTrigger
              onClick={(e) => e.stopPropagation()}
              className="flex size-5 shrink-0 items-center justify-center rounded-sm text-muted-foreground hover:bg-muted hover:text-foreground"
            >
              <ChevronRight className={cn("size-3.5 transition-transform", open && "rotate-90")} />
            </CollapsibleTrigger>
          ) : (
            <span className="size-5 shrink-0" />
          )}
          <button
            type="button"
            onClick={() => onSelectNoteType(noteType)}
            className={cn(
              "flex flex-1 items-center justify-between rounded-sm px-2 py-1.5 text-left text-sm hover:bg-muted",
              isSelected && "bg-accent text-accent-foreground hover:bg-accent",
            )}
          >
            <span className="truncate">{noteType.name}</span>
            <span className="text-xs text-muted-foreground">{Number(noteType.note_count)}</span>
          </button>
          {canManage ? (
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <Button variant="ghost" size="icon-sm" aria-label={`Actions for ${noteType.name}`} onClick={(e) => e.stopPropagation()} />
                }
              >
                <EllipsisVertical />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" className="w-44">
                <DropdownMenuGroup>
                  <DropdownMenuLabel>{noteType.name}</DropdownMenuLabel>
                </DropdownMenuGroup>
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={(e) => { e.stopPropagation(); onAddTemplate(); }}>Add template</DropdownMenuItem>
                <DropdownMenuItem onClick={(e) => { e.stopPropagation(); onDuplicate(); }}>Duplicate</DropdownMenuItem>
                <DropdownMenuItem variant="destructive" onClick={(e) => { e.stopPropagation(); onDelete(); }}>Delete</DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          ) : null}
        </div>
        {hasChildren ? (
          <CollapsibleContent>
            <ul className="ml-2.5 flex flex-col border-l border-border pl-1">
              {noteType.templates.map((t, i) => {
                const isTmplSelected = isSelected && selectedTemplateId === t.id
                return (
                  <li key={String(t.id)} className="group/row flex items-center">
                    <button
                      type="button"
                      onClick={() => onSelectTemplate(noteType, t.id)}
                      className={cn(
                        "flex flex-1 items-center rounded-sm px-2 py-1 text-left text-sm hover:bg-muted",
                        isTmplSelected && "bg-accent text-accent-foreground hover:bg-accent",
                      )}
                    >
                      <span className="truncate">{t.name}</span>
                    </button>
                    {canManage ? (
                      <span className="flex shrink-0 items-center opacity-0 group-hover/row:opacity-100">
                        <Button variant="ghost" size="icon-sm" aria-label="Move up" disabled={i === 0} onClick={() => onMoveTemplate(i, -1)}><ChevronUp /></Button>
                        <Button variant="ghost" size="icon-sm" aria-label="Move down" disabled={i === noteType.templates.length - 1} onClick={() => onMoveTemplate(i, 1)}><ChevronDown /></Button>
                        <Button variant="ghost" size="icon-sm" aria-label="Delete template" onClick={() => onDeleteTemplate(t.id)}><X /></Button>
                      </span>
                    ) : null}
                  </li>
                )
              })}
            </ul>
          </CollapsibleContent>
        ) : null}
      </li>
    </Collapsible>
  )
}

// Panel 1 — note-type CRUD (name, fields, sort field).
function NoteTypePanel({ noteType, canManage, update }: { noteType: NoteTypeView; canManage: boolean; update: (e: Event) => void }) {
  const [name, setName] = useState(noteType.name)
  const [fieldNames, setFieldNames] = useState(noteType.field_names.join(", "))
  const [sortField, setSortField] = useState(noteType.sort_field)

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
        <Button onClick={save} disabled={!dirty}>Save</Button>
      ) : null}
    </div>
  )
}

// Panel 3 — template editor. Tabs on top, textarea fills the space, save at the bottom.
function TemplatePanel({ noteType, template, canManage, update }: { noteType: NoteTypeView; template: TemplateView | null; canManage: boolean; update: (e: Event) => void }) {
  const [tab, setTab] = useState<string>("styling")
  const [styling, setStyling] = useState(noteType.styling)
  const [front, setFront] = useState(template?.front_pattern ?? "")
  const [back, setBack] = useState(template?.back_pattern ?? "")

  useEffect(() => {
    setStyling(noteType.styling)
  }, [noteType.id, noteType.styling])
  useEffect(() => {
    setFront(template?.front_pattern ?? "")
    setBack(template?.back_pattern ?? "")
  }, [template?.id])

  const dirty =
    tab === "styling"
      ? styling !== noteType.styling
      : tab === "front"
        ? template !== null && front !== template.front_pattern
        : template !== null && back !== template.back_pattern

  const value = tab === "styling" ? styling : tab === "front" ? front : back

  function save() {
    if (tab === "styling") {
      update(eventNoteTypeUpdate(noteType.id, null, null, null, styling))
    } else if (tab === "front" && template) {
      update(eventNoteTypeTemplateUpdate(noteType.id, template.id, null, front, null))
    } else if (tab === "back" && template) {
      update(eventNoteTypeTemplateUpdate(noteType.id, template.id, null, null, back))
    }
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {/* Tabs at the top */}
      <div className="shrink-0 border-b px-4 py-2">
        <Tabs value={tab} onValueChange={(v) => { if (v !== null) setTab(v) }}>
          <TabsList>
            <TabsTrigger value="styling">Styling</TabsTrigger>
            <TabsTrigger value="front">Front</TabsTrigger>
            <TabsTrigger value="back">Back</TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      {/* Textarea fills the remaining space */}
      <div className="min-h-0 flex-1 p-4">
        <div className="flex h-full flex-col gap-1.5">
          <Label className="text-xs text-muted-foreground">
            {tab === "styling"
              ? "Styling (CSS, shared by all templates)"
              : tab === "front"
                ? `${template ? template.name : "Front"} — use {{Field}} placeholders`
                : `${template ? template.name : "Back"} — use {{Field}} placeholders`}
          </Label>
          <Textarea
            value={value}
            onChange={(e) => {
              if (tab === "styling") setStyling(e.target.value)
              else if (tab === "front") setFront(e.target.value)
              else setBack(e.target.value)
            }}
            disabled={!canManage || (tab !== "styling" && !template)}
            className="min-h-0 flex-1 resize-none font-mono text-sm"
          />
        </div>
      </div>

      {/* Save at the bottom */}
      <div className="shrink-0 border-t px-4 py-2">
        {canManage && (tab === "styling" || template) ? (
          <Button onClick={save} disabled={!dirty}>Save</Button>
        ) : null}
      </div>
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

function CreateTemplateDialog({ noteType, update, open, onOpenChange }: { noteType: NoteTypeView | null; update: (e: Event) => void; open: boolean; onOpenChange: (o: boolean) => void }) {
  const [name, setName] = useState("")
  const [front, setFront] = useState("")
  const [back, setBack] = useState("")

  useEffect(() => {
    if (open) {
      setName("")
      setFront("")
      setBack("")
    }
  }, [open])

  function submit() {
    if (!noteType) return
    const n = name.trim()
    if (n === "") return
    update(eventNoteTypeTemplateCreate(noteType.id, n, front, back))
    onOpenChange(false)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
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
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button onClick={submit} disabled={name.trim() === ""}>Create</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function DeleteNoteTypeDialog({ noteType, open, onOpenChange, update }: { noteType: NoteTypeView | null; open: boolean; onOpenChange: (o: boolean) => void; update: (e: Event) => void }) {
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
          <Button variant="destructive" onClick={() => { if (noteType) update(eventNoteTypeDelete(noteType.id)); onOpenChange(false); }}>Delete</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
