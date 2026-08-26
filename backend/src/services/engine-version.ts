/**
 * Engine algorithm version, embedded into cached chart_data.
 * Bump whenever a calculation algorithm changes (ziwei or western) so that
 * cached interpretations are invalidated and recalculated on next GET.
 */
export const ENGINE_VERSION = '2.0.0';
