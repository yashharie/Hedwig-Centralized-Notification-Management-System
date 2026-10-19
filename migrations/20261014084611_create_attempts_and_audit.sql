CREATE TABLE delivery_attempts (
    id              BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    job_id          BIGINT UNSIGNED NOT NULL,
    attempt_number  INT UNSIGNED    NOT NULL,
    outcome         VARCHAR(20)     NOT NULL,
    error_code      VARCHAR(50)     NULL,
    error_message   TEXT            NULL,
    started_at      TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at     TIMESTAMP       NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_attempt_per_job (job_id, attempt_number),
    CONSTRAINT fk_attempts_job FOREIGN KEY (job_id)
        REFERENCES delivery_jobs (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE audit_logs (
    id              BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    entity_type     VARCHAR(30)     NOT NULL,
    entity_id       BIGINT UNSIGNED NOT NULL,
    event_type      VARCHAR(50)     NOT NULL,
    details         JSON            NULL,
    created_at      TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_audit_entity (entity_type, entity_id, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;