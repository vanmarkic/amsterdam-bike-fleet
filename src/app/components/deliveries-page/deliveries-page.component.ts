import { Component, OnInit, OnDestroy, ChangeDetectionStrategy, ChangeDetectorRef } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Subject } from 'rxjs';
import { takeUntil, finalize } from 'rxjs/operators';
import { Delivery, DeliveryStatus } from '../../models/fleet.models';
import { DeliveryService } from '../../services/delivery.service';

type FilterStatus = 'all' | DeliveryStatus;

@Component({
  selector: 'app-deliveries-page',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './deliveries-page.component.html',
  styleUrls: ['./deliveries-page.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class DeliveriesPageComponent implements OnInit, OnDestroy {
  deliveries: Delivery[] = [];
  filteredDeliveries: Delivery[] = [];
  selectedDelivery: Delivery | null = null;
  selectedStatus: FilterStatus = 'all';
  loading = false;
  error: string | null = null;

  readonly statusOptions: { value: FilterStatus; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'completed', label: 'Completed' },
    { value: 'ongoing', label: 'Ongoing' },
    { value: 'upcoming', label: 'Upcoming' }
  ];

  private readonly destroy$ = new Subject<void>();

  constructor(
    private deliveryService: DeliveryService,
    private cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.loadDeliveries();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  loadDeliveries(): void {
    this.loading = true;
    this.error = null;

    this.deliveryService.getDeliveries()
      .pipe(
        takeUntil(this.destroy$),
        finalize(() => {
          this.loading = false;
          this.cdr.markForCheck();
        })
      )
      .subscribe({
        next: (deliveries) => {
          this.deliveries = deliveries;
          this.applyFilter();
        },
        error: (err) => {
          this.error = 'Failed to load deliveries. Please try again.';
          console.error('Error loading deliveries:', err);
        }
      });
  }

  onStatusFilterChange(status: FilterStatus): void {
    this.selectedStatus = status;
    this.applyFilter();
    this.cdr.markForCheck();
  }

  selectDelivery(delivery: Delivery): void {
    this.selectedDelivery = delivery;
    this.cdr.markForCheck();
  }

  trackByDeliveryId(_index: number, delivery: Delivery): string {
    return delivery.id;
  }

  getStars(rating: number | null): string {
    if (rating === null) {
      return '';
    }
    const fullStars = Math.floor(rating);
    const emptyStars = 5 - fullStars;
    return '\u2605'.repeat(fullStars) + '\u2606'.repeat(emptyStars);
  }

  formatDate(date: Date | null): string {
    if (!date) {
      return '-';
    }
    const d = new Date(date);
    return d.toLocaleString('en-GB', {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  }

  private applyFilter(): void {
    if (this.selectedStatus === 'all') {
      this.filteredDeliveries = [...this.deliveries];
    } else {
      this.filteredDeliveries = this.deliveries.filter(
        d => d.status === this.selectedStatus
      );
    }

    // Clear selection if filtered out
    if (this.selectedDelivery && !this.filteredDeliveries.find(d => d.id === this.selectedDelivery?.id)) {
      this.selectedDelivery = null;
    }
  }
}
