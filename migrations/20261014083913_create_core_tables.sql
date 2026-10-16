CREATE TABLE notification_batches (
    id              BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    application_id  BIGINT UNSIGNED NOT NULL,
    status          VARCHAR(20)     NOT NULL DEFAULT 'queued',
    total_count     INT UNSIGNED    NOT NULL DEFAULT 0,
    created_at      TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP
                                    ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_batches_app (application_id),
    CONSTRAINT fk_batches_app FOREIGN KEY (application_id)
        REFERENCES applications (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE notifications (
    id              BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    application_id  BIGINT UNSIGNED NOT NULL,
    batch_id        BIGINT UNSIGNED NULL,
    idempotency_key VARCHAR(100)    NULL,
    recipient       VARCHAR(255)    NOT NULL,
    subject         VARCHAR(255)    NULL,
    body            TEXT            NOT NULL,
    status          VARCHAR(20)     NOT NULL DEFAULT 'queued',
    created_at      TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP
                                    ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_notifications_idem (application_id, idempotency_key),
    KEY idx_notifications_batch (batch_id),
    CONSTRAINT fk_notifications_app FOREIGN KEY (application_id)
        REFERENCES applications (id),
    CONSTRAINT fk_notifications_batch FOREIGN KEY (batch_id)
        REFERENCES notification_batches (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE delivery_jobs (
    id               BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    notification_id  BIGINT UNSIGNED NOT NULL,
    channel          VARCHAR(20)     NOT NULL,
    status           VARCHAR(20)     NOT NULL DEFAULT 'queued',
    attempt_count    INT UNSIGNED    NOT NULL DEFAULT 0,
    max_attempts     INT UNSIGNED    NOT NULL DEFAULT 5,
    next_attempt_at  TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP,
    locked_by        VARCHAR(100)    NULL,
    lease_token      VARCHAR(64)     NULL,
    lease_expires_at TIMESTAMP       NULL,
    last_error       TEXT            NULL,
    created_at       TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP
                                     ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_job_per_channel (notification_id, channel),
    KEY idx_jobs_claim (status, next_attempt_at),
    KEY idx_jobs_lease (status, lease_expires_at),
    CONSTRAINT fk_jobs_notification FOREIGN KEY (notification_id)
        REFERENCES notifications (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;