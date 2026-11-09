// What a screen may put in the shell's top bar: elements into `slot`, and the title (undefined gives the bar back its default).
export interface BarContext { slot: HTMLElement | null; setTitle(title: string | undefined): void }
