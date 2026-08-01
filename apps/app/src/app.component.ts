import { Component } from '@angular/core';

import { GameShellComponent } from '@rusty-roguelike/feature-game';

@Component({
  imports: [GameShellComponent],
  selector: 'rr-root',
  standalone: true,
  template: `<rr-game-shell />`,
})
export class AppComponent {}
