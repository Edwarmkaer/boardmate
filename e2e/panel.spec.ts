import { expect, test } from "@playwright/test";

test("direct controls update simulated devices and survive panel collapse", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "boardmate", exact: true })).toBeVisible();
  await page.getByRole("tab", { name: "Dispositivos" }).click();
  const board = page.getByRole("region", { name: "Mesa de trabajo" });
  await expect(board.getByText("Simulado", { exact: true })).toBeVisible();
  await board.getByRole("button", { name: "Encender luz" }).click();
  await expect(board.getByText("Luz encendida", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Contraer panel" }).click();
  await page.getByRole("button", { name: "Abrir boardmate" }).click();
  await expect(board.getByText("Luz encendida", { exact: true })).toBeVisible();
});

test("assistant command updates actual simulator state and rejects invalid angles", async ({
  page,
}) => {
  await page.goto("/");
  const input = page.getByRole("textbox", { name: "Mensaje para boardmate" });
  await input.fill("Gira el servo a 45 grados");
  await page.getByRole("button", { name: "Enviar mensaje" }).click();
  await expect(page.getByRole("log")).toContainText("Servo simulado ajustado a 45°");
  await input.fill("Gira el servo a 400 grados");
  await page.getByRole("button", { name: "Enviar mensaje" }).click();
  await expect(page.getByRole("alert", { name: "Error de operación" })).toContainText("angle");
  await page.getByRole("tab", { name: "Dispositivos" }).click();
  await expect(page.getByRole("slider", { name: "Ángulo del servo" })).toHaveValue("45");
  await expect
    .poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth))
    .toBe(true);
});
