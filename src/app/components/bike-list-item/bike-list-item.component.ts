import { Component, Input, Output, EventEmitter, ChangeDetectionStrategy } from '@angular/core';
import { BikePosition } from '../../models/fleet.models';

@Component({
  selector: 'app-bike-list-item',
  templateUrl: './bike-list-item.component.html',
  styleUrls: ['./bike-list-item.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class BikeListItemComponent {
  @Input() bike!: BikePosition;
  @Input() isSelected = false;
  @Output() bikeSelected = new EventEmitter<BikePosition>();

  onSelect(): void {
    this.bikeSelected.emit(this.bike);
  }
}
