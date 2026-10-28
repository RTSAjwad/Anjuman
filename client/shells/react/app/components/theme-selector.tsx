"use client"

import { Sun, Moon, SunMoon } from "lucide-react";
import { useTheme } from "../lib/theme-provider";
import { Button } from "./ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuLabel,
} from "./ui/dropdown-menu";

const OPTIONS = [
  { value: "light", label: "Light", Icon: Sun },
  { value: "dark", label: "Dark", Icon: Moon },
  { value: "system", label: "Automatic", Icon: SunMoon },
] as const;

export function ThemeSelector() {
  const { theme, setTheme } = useTheme();

  const ValueIcon = OPTIONS.find((o) => o.value === theme)?.Icon ?? Sun;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button variant="ghost" size="icon-sm" aria-label="Theme">
            <ValueIcon />
          </Button>
        }
      />
      <DropdownMenuContent align="start" className="w-40">
        <DropdownMenuLabel>Theme</DropdownMenuLabel>
        <DropdownMenuRadioGroup
          value={theme}
          onValueChange={(v) => setTheme(v as "light" | "dark" | "system")}
        >
          {OPTIONS.map(({ value, label, Icon }) => (
            <DropdownMenuRadioItem key={value} value={value}>
              <Icon />
              {label}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
