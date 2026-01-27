import { Component, OnInit } from '@angular/core';
import { Router, NavigationStart, NavigationEnd, NavigationCancel, NavigationError, GuardsCheckStart, GuardsCheckEnd } from '@angular/router';

@Component({
  selector: 'app-root',
  templateUrl: './app.component.html',
  styleUrls: ['./app.component.scss']
})
export class AppComponent implements OnInit {
  title = 'amsterdam-bike-fleet';

  constructor(private router: Router) {}

  ngOnInit(): void {
    // Debug router events
    this.router.events.subscribe(event => {
      if (event instanceof NavigationStart) {
        console.log('[Router] NavigationStart:', event.url);
      }
      if (event instanceof GuardsCheckStart) {
        console.log('[Router] GuardsCheckStart');
      }
      if (event instanceof GuardsCheckEnd) {
        console.log('[Router] GuardsCheckEnd, shouldActivate:', event.shouldActivate);
      }
      if (event instanceof NavigationEnd) {
        console.log('[Router] NavigationEnd:', event.url);
      }
      if (event instanceof NavigationCancel) {
        console.log('[Router] NavigationCancel:', event.url, 'reason:', event.reason);
      }
      if (event instanceof NavigationError) {
        console.log('[Router] NavigationError:', event.url, 'error:', event.error);
      }
    });
  }
}
