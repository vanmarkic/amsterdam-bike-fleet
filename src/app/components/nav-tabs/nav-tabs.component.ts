import { Component, ChangeDetectionStrategy } from '@angular/core';

interface NavTab {
  label: string;
  route: string;
}

@Component({
  selector: 'app-nav-tabs',
  templateUrl: './nav-tabs.component.html',
  styleUrls: ['./nav-tabs.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class NavTabsComponent {
  readonly tabs: NavTab[] = [
    { label: 'Map', route: '/map' },
    { label: 'Deliveries', route: '/deliveries' },
    { label: 'Issues', route: '/issues' }
  ];

  showLicenseDialog = false;

  openLicenseDialog(): void {
    this.showLicenseDialog = true;
  }
}
