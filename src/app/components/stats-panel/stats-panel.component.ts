import { Component, Input, ChangeDetectionStrategy } from '@angular/core';

@Component({
  selector: 'app-stats-panel',
  templateUrl: './stats-panel.component.html',
  styleUrls: ['./stats-panel.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class StatsPanelComponent {
  @Input() bikeCount = 0;
  @Input() deliveringCount = 0;
  @Input() lastUpdate: Date | null = null;
}
