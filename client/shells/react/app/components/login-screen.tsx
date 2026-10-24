import { useState } from "react";
import { eventLoginSubmit } from "shared_types/app";
import type { Event } from "shared_types/app";
import { useCore } from "../lib/core-provider.client";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Label } from "./ui/label";
import { Input } from "./ui/input";
import { Button } from "./ui/button";

export function LoginScreen() {
  const { view, update } = useCore();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  const busy = view?.busy ?? false;
  const error = view?.error ?? undefined;

  function submit(e: React.FormEvent) {
    e.preventDefault();
    update(eventLoginSubmit(email, password));
  }

  return (
    <div className="flex min-h-dvh items-center justify-center p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>Sign in</CardTitle>
          <CardDescription>Enter your Anjuman credentials.</CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={submit} className="flex flex-col gap-4">
            <div className="flex flex-col gap-2">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                type="email"
                autoComplete="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="name@school.edu"
                required
              />
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                autoComplete="current-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
            </div>
            {error ? (
              <p className="text-sm text-destructive" role="alert">
                {error}
              </p>
            ) : null}
            <Button type="submit" disabled={busy}>
              {busy ? "Signing in…" : "Sign in"}
            </Button>
            <div className="flex flex-col gap-2 border-t pt-4">
              <p className="text-xs text-muted-foreground">Quick login (dev seed):</p>
              <div className="grid grid-cols-3 gap-2">
                <QuickLogin label="Teacher" email="teacher@school1.com" password="teach123" update={update} />
                <QuickLogin label="Admin" email="admin@school1.com" password="admin123" update={update} />
                <QuickLogin label="Student" email="student@school1.com" password="stud123" update={update} />
              </div>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

function QuickLogin({
  label,
  email,
  password,
  update,
}: {
  label: string;
  email: string;
  password: string;
  update: (event: Event) => void;
}) {
  return (
    <Button
      type="button"
      variant="outline"
      onClick={() => update(eventLoginSubmit(email, password))}
    >
      {label}
    </Button>
  );
}
