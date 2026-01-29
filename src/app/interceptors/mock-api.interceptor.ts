import { Injectable } from '@angular/core';
import { HttpRequest, HttpHandler, HttpEvent, HttpInterceptor, HttpResponse } from '@angular/common/http';
import { Observable, of, throwError } from 'rxjs';
import { delay } from 'rxjs/operators';
import {
  Delivery,
  Issue,
  DeliveryStatus,
  IssueReporterType,
  IssueCategory
} from '../models/fleet.models';

@Injectable()
export class MockApiInterceptor implements HttpInterceptor {
  private deliveries: Delivery[] = [];
  private issues: Issue[] = [];
  private initialized = false;

  // Simple response cache (key -> { data, timestamp })
  private cache = new Map<string, { data: unknown; timestamp: number }>();
  private readonly CACHE_TTL_MS = 30000; // 30 second cache for list endpoints

  // Dutch names for realistic data
  private readonly CUSTOMER_NAMES = [
    'P. de Vries', 'M. Jansen', 'A. Bakker', 'J. van Dijk', 'S. Visser',
    'L. Smit', 'K. Mulder', 'R. de Boer', 'T. Bos', 'E. van den Berg',
    'H. Dekker', 'F. Vermeer', 'B. van Leeuwen', 'N. Kok', 'D. Peters'
  ];

  private readonly RESTAURANT_NAMES = [
    'De Pizzabakker', 'Wok to Walk', 'Febo', 'New York Pizza', 'Dominos',
    'Thai Express', 'Sushi Time', 'Burger King', 'McDonalds', 'Subway',
    'La Place', 'Vapiano', 'Bagels & Beans', 'De Italiaan', 'Ramen Ya'
  ];

  private readonly AMSTERDAM_STREETS = [
    'Damrak', 'Rokin', 'Kalverstraat', 'Leidsestraat', 'Utrechtsestraat',
    'Overtoom', 'Kinkerstraat', 'De Pijp', 'Jordaan', 'Plantage',
    'Oost', 'West', 'Noord', 'Zuid', 'Centrum'
  ];

  private readonly COMPLAINT_TEXTS = [
    'Order arrived cold',
    'Missing items in order',
    'Delivery took too long',
    'Wrong order delivered',
    'Food was damaged',
    'Rider was rude',
    'Packaging was open'
  ];

  private readonly ISSUE_DESCRIPTIONS: Record<IssueCategory, string[]> = {
    late: [
      'Delivery arrived 30 minutes late',
      'Order was delayed without notification',
      'Expected delivery time was not met'
    ],
    damaged: [
      'Food container was crushed',
      'Drinks spilled in the bag',
      'Packaging was torn'
    ],
    wrong_order: [
      'Received someone elses order',
      'Items were missing from order',
      'Order had wrong items'
    ],
    rude: [
      'Deliverer was impolite',
      'Bad attitude at handover',
      'Unprofessional behavior'
    ],
    bike_problem: [
      'Flat tire during delivery',
      'Brake issues reported',
      'Chain broke mid-route'
    ],
    other: [
      'General complaint',
      'Feedback about service',
      'Suggestion for improvement'
    ]
  };

  constructor() {
    this.initializeMockData();
  }

  private initializeMockData(): void {
    if (this.initialized) return;
    this.initialized = true;

    // Generate 50 deliveries across 20 bikes
    const bikeIds = Array.from({ length: 20 }, (_, i) => `bike-${i + 1}`);

    for (let i = 0; i < 50; i++) {
      const bikeId = bikeIds[i % 20];
      const status = this.randomDeliveryStatus();
      const createdAt = this.randomDate(7); // within last 7 days

      const delivery: Delivery = {
        id: `delivery-${i + 1}`,
        bikeId,
        status,
        customerName: this.randomItem(this.CUSTOMER_NAMES),
        customerAddress: `${this.randomItem(this.AMSTERDAM_STREETS)} ${Math.floor(Math.random() * 200) + 1}`,
        restaurantName: this.randomItem(this.RESTAURANT_NAMES),
        restaurantAddress: `${this.randomItem(this.AMSTERDAM_STREETS)} ${Math.floor(Math.random() * 200) + 1}`,
        rating: status === 'completed' && Math.random() < 0.3 ? Math.floor(Math.random() * 5) + 1 : null,
        complaint: status === 'completed' && Math.random() < 0.1 ? this.randomItem(this.COMPLAINT_TEXTS) : null,
        createdAt,
        completedAt: status === 'completed' ? new Date(createdAt.getTime() + Math.random() * 3600000) : null
      };

      this.deliveries.push(delivery);
    }

    // Generate 20 issues
    for (let i = 0; i < 20; i++) {
      const hasDelivery = Math.random() < 0.7;
      const deliveryId = hasDelivery
        ? this.deliveries[Math.floor(Math.random() * this.deliveries.length)].id
        : null;
      const bikeId = hasDelivery && deliveryId
        ? this.deliveries.find(d => d.id === deliveryId)!.bikeId
        : `bike-${Math.floor(Math.random() * 20) + 1}`;
      const category = this.randomIssueCategory();

      const issue: Issue = {
        id: `issue-${i + 1}`,
        deliveryId,
        bikeId,
        reporterType: this.randomReporterType(),
        category,
        description: this.randomItem(this.ISSUE_DESCRIPTIONS[category]),
        resolved: Math.random() < 0.4,
        createdAt: this.randomDate(14) // within last 14 days
      };

      this.issues.push(issue);
    }
  }

  intercept(request: HttpRequest<unknown>, next: HttpHandler): Observable<HttpEvent<unknown>> {
    const url = request.url;
    const method = request.method;

    // Simulate 5% error rate
    if (Math.random() < 0.05) {
      return throwError(() => new Error('Simulated network error')).pipe(
        delay(this.randomDelay())
      );
    }

    // Route matching
    if (url.match(/\/api\/deliveries\/[\w-]+$/) && method === 'GET') {
      return this.getDeliveryById(url);
    }
    if (url.includes('/api/deliveries') && method === 'GET') {
      return this.getDeliveries(request);
    }
    if (url.match(/\/api\/issues\/[\w-]+$/) && method === 'GET') {
      return this.getIssueById(url);
    }
    if (url.includes('/api/issues') && method === 'GET') {
      return this.getIssues(request);
    }

    // Pass through other requests
    return next.handle(request);
  }

  private getDeliveries(request: HttpRequest<unknown>): Observable<HttpEvent<Delivery[]>> {
    const cacheKey = `deliveries:${request.params.toString()}`;
    const cached = this.getFromCache<Delivery[]>(cacheKey);
    if (cached) {
      // Return cached data immediately (no delay)
      return of(new HttpResponse({ status: 200, body: cached }));
    }

    let filtered = [...this.deliveries];

    const status = request.params.get('status');
    const bikeId = request.params.get('bikeId');

    if (status) {
      filtered = filtered.filter(d => d.status === status);
    }
    if (bikeId) {
      filtered = filtered.filter(d => d.bikeId === bikeId);
    }

    // Sort by createdAt descending
    filtered.sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime());

    // Cache the result
    this.setCache(cacheKey, filtered);

    return of(new HttpResponse({ status: 200, body: filtered })).pipe(
      delay(this.randomDelay(200, 400))
    );
  }

  private getDeliveryById(url: string): Observable<HttpEvent<Delivery>> {
    const id = url.split('/').pop();
    const delivery = this.deliveries.find(d => d.id === id);

    if (!delivery) {
      return throwError(() => new Error('Delivery not found')).pipe(
        delay(this.randomDelay(100, 200))
      );
    }

    return of(new HttpResponse({ status: 200, body: delivery })).pipe(
      delay(this.randomDelay(100, 200))
    );
  }

  private getIssues(request: HttpRequest<unknown>): Observable<HttpEvent<Issue[]>> {
    const cacheKey = `issues:${request.params.toString()}`;
    const cached = this.getFromCache<Issue[]>(cacheKey);
    if (cached) {
      return of(new HttpResponse({ status: 200, body: cached }));
    }

    let filtered = [...this.issues];

    const resolved = request.params.get('resolved');
    const bikeId = request.params.get('bikeId');
    const category = request.params.get('category');

    if (resolved !== null) {
      filtered = filtered.filter(i => i.resolved === (resolved === 'true'));
    }
    if (bikeId) {
      filtered = filtered.filter(i => i.bikeId === bikeId);
    }
    if (category) {
      filtered = filtered.filter(i => i.category === category);
    }

    // Sort by createdAt descending
    filtered.sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime());

    this.setCache(cacheKey, filtered);

    return of(new HttpResponse({ status: 200, body: filtered })).pipe(
      delay(this.randomDelay(200, 400))
    );
  }

  private getIssueById(url: string): Observable<HttpEvent<Issue>> {
    const id = url.split('/').pop();
    const issue = this.issues.find(i => i.id === id);

    if (!issue) {
      return throwError(() => new Error('Issue not found')).pipe(
        delay(this.randomDelay(100, 200))
      );
    }

    return of(new HttpResponse({ status: 200, body: issue })).pipe(
      delay(this.randomDelay(100, 200))
    );
  }

  // Helper methods
  private randomDeliveryStatus(): DeliveryStatus {
    const rand = Math.random();
    if (rand < 0.6) return 'completed';
    if (rand < 0.85) return 'ongoing';
    return 'upcoming';
  }

  private randomReporterType(): IssueReporterType {
    const rand = Math.random();
    if (rand < 0.5) return 'customer';
    if (rand < 0.8) return 'deliverer';
    return 'restaurant';
  }

  private randomIssueCategory(): IssueCategory {
    const rand = Math.random();
    if (rand < 0.4) return 'late';
    if (rand < 0.6) return 'wrong_order';
    if (rand < 0.75) return 'damaged';
    if (rand < 0.9) return 'bike_problem';
    if (rand < 0.95) return 'rude';
    return 'other';
  }

  private randomItem<T>(array: T[]): T {
    return array[Math.floor(Math.random() * array.length)];
  }

  private randomDate(daysAgo: number): Date {
    const now = new Date();
    const past = new Date(now.getTime() - Math.random() * daysAgo * 24 * 60 * 60 * 1000);
    return past;
  }

  private randomDelay(min = 200, max = 400): number {
    return Math.floor(Math.random() * (max - min)) + min;
  }

  // Cache helpers
  private getFromCache<T>(key: string): T | null {
    const entry = this.cache.get(key);
    if (!entry) return null;
    if (Date.now() - entry.timestamp > this.CACHE_TTL_MS) {
      this.cache.delete(key);
      return null;
    }
    return entry.data as T;
  }

  private setCache(key: string, data: unknown): void {
    this.cache.set(key, { data, timestamp: Date.now() });
  }
}
