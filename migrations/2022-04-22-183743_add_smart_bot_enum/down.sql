-- ALTER TYPE bot_type DELETE VALUE 'SMART';
-- TODO no permissions
DELETE FROM pg_enum
WHERE enumlabel = 'SMART'
  AND enumtypid = (
    SELECT oid FROM pg_type WHERE typname = 'bot_type'
)