import { Component, Input, Output, EventEmitter, ChangeDetectionStrategy } from '@angular/core';
import { BikePosition } from '../../models/fleet.models';

@Component({
  selector: 'app-bike-list-panel',
  templateUrl: './bike-list-panel.component.html',
  styleUrls: ['./bike-list-panel.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class BikeListPanelComponent {
  @Input() bikes: BikePosition[] = [];
  @Input() selectedBikeId: string | null = null;
  @Output() bikeSelected = new EventEmitter<BikePosition>();

  trackByBikeId(_index: number, bike: BikePosition): string {
    return bike.id;
  }

  onBikeSelected(bike: BikePosition): void {
    this.bikeSelected.emit(bike);
  }
}
