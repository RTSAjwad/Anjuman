"use client"

import * as React from "react"
import { cn } from "cn"
import { XIcon } from "lucide-react"
import { Badge } from "~/components/ui/badge"

// A self-built chip input for note tags (shadcn doesn't ship one). Chips are
// rendered as removable Badges; typing + Enter commits a new chip, Backspace on
// an empty input removes the last one. (Autocomplete from the tag namespace was
// deferred — see the note-tag carve-out docs.)
//
// It is fully controlled: the caller owns `tags` and supplies `onAdd`/`onRemove`
// (which, for notes, map to the core's NoteAddTags/NoteRemoveTags events — the
// chip input itself holds no authoritative tag state).

type ChipInputProps = {
  tags: string[]
  onAdd: (tag: string) => void
  onRemove: (tag: string) => void
  disabled?: boolean
  placeholder?: string
  "aria-label"?: string
  className?: string
}

export function ChipInput({
  tags,
  onAdd,
  onRemove,
  disabled,
  placeholder = "Add tag…",
  "aria-label": ariaLabel,
  className,
}: ChipInputProps) {
  const [value, setValue] = React.useState("")

  // Commit the trimmed input as a new tag if it's non-empty and not a duplicate.
  function commit(raw: string) {
    const tag = raw.trim()
    if (!tag) return
    if (tags.includes(tag)) {
      setValue("")
      return
    }
    onAdd(tag)
    setValue("")
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault()
      commit(value)
    } else if (e.key === "Backspace" && value === "" && tags.length > 0) {
      onRemove(tags[tags.length - 1])
    } else if (e.key === "Escape") {
      setValue("")
    }
  }

  return (
    <div className={cn("flex flex-col gap-1.5", className)}>
      <div
        className={cn(
          "flex min-h-9 flex-wrap items-center gap-1.5 rounded-lg border border-input px-2 py-1",
          "focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/50",
          disabled && "pointer-events-none opacity-50",
        )}
      >
        {tags.map((tag) => (
          <Badge key={tag} variant="secondary" className="gap-1">
            {tag}
            {!disabled && (
              <button
                type="button"
                aria-label={`Remove tag ${tag}`}
                onClick={() => onRemove(tag)}
                className="rounded-sm text-muted-foreground hover:text-foreground focus-visible:ring-1 focus-visible:ring-ring"
              >
                <XIcon className="size-3" />
              </button>
            )}
          </Badge>
        ))}
        <input
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={onKeyDown}
          disabled={disabled}
          placeholder={tags.length === 0 ? placeholder : undefined}
          aria-label={ariaLabel ?? "Tag"}
          className="h-6 min-w-16 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
        />
      </div>
    </div>
  )
}
